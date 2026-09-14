use crate::model::{AppSettings, LayoutProfile, Snapshot, WidgetContent, WidgetInstance};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::{Path, PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};

pub const BACKUP_FORMAT: &str = "desktop-dashboard-backup";
const BACKUP_VERSION: u32 = 2;
const MAX_BACKUP_BYTES: u64 = 32 * 1024 * 1024;
const RETAINED_STANDARD_BACKUPS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupData {
    pub settings: AppSettings,
    pub widgets: Vec<WidgetInstance>,
    pub content: BTreeMap<String, WidgetContent>,
    #[serde(default)] pub active_layout_key: Option<String>,
    #[serde(default)] pub layout_profiles: BTreeMap<String, LayoutProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupDocument {
    pub format: String,
    pub version: u32,
    pub created_at: u64,
    pub app_version: String,
    pub data: BackupData,
}

impl BackupDocument {
    pub fn from_snapshot(snapshot: &Snapshot, created_at: u64) -> Self {
        Self {
            format: BACKUP_FORMAT.into(), version: BACKUP_VERSION, created_at,
            app_version: env!("CARGO_PKG_VERSION").into(),
            data: BackupData { settings: snapshot.settings.clone(), widgets: snapshot.widgets.clone(), content: snapshot.content.clone(),
                active_layout_key: snapshot.active_layout_key.clone(), layout_profiles: snapshot.layout_profiles.clone() },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus { pub last_backup_at: Option<u64>, pub last_backup_path: Option<String> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOperation { pub status: String, pub path: Option<String>, pub created_at: Option<u64> }

pub fn now_seconds() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
fn backup_directory(data_dir: &Path) -> PathBuf { data_dir.join("backups") }

pub fn parse_document_bytes(bytes: &[u8]) -> Result<BackupDocument, String> {
    if bytes.len() as u64 > MAX_BACKUP_BYTES { return Err("backup_too_large".into()); }
    let document: BackupDocument = serde_json::from_slice(bytes).map_err(|_| "backup_invalid".to_string())?;
    if document.format != BACKUP_FORMAT || !(1..=BACKUP_VERSION).contains(&document.version) { return Err("backup_invalid".into()); }
    Ok(document)
}

pub fn snapshot_from_document(document: BackupDocument, current: &Snapshot) -> Result<Snapshot, String> {
    let mut candidate = Snapshot {
        revision: current.revision.checked_add(1).ok_or_else(|| "backup_invalid".to_string())?,
        settings: document.data.settings, widgets: document.data.widgets, content: document.data.content,
        providers: current.providers.clone(),
        active_layout_key: document.data.active_layout_key, layout_profiles: document.data.layout_profiles,
    };
    crate::model::normalize_snapshot(&mut candidate).map_err(|_| "backup_invalid".to_string())?;
    Ok(candidate)
}

fn write_document(path: &Path, snapshot: &Snapshot, created_at: u64) -> Result<(), String> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|_| "backup_write_failed")?; }
    let document = BackupDocument::from_snapshot(snapshot, created_at);
    let bytes = serde_json::to_vec_pretty(&document).map_err(|_| "backup_write_failed")?;
    let mut file = fs::File::create(path).map_err(|_| "backup_write_failed")?;
    file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|_| "backup_write_failed".to_string())
}

pub fn write_standard_backup(data_dir: &Path, snapshot: &Snapshot) -> Result<BackupOperation, String> {
    let created_at = now_seconds();
    let directory = backup_directory(data_dir);
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let path = directory.join(format!("desktop-dashboard-backup-{unique}.json"));
    write_document(&path, snapshot, created_at)?;
    retain_standard_backups(&directory, RETAINED_STANDARD_BACKUPS)?;
    Ok(BackupOperation { status: "saved".into(), path: Some(path.to_string_lossy().into_owned()), created_at: Some(created_at) })
}

pub fn export_to_path(path: &Path, snapshot: &Snapshot) -> Result<BackupOperation, String> {
    let created_at = now_seconds();
    write_document(path, snapshot, created_at)?;
    Ok(BackupOperation { status: "saved".into(), path: Some(path.to_string_lossy().into_owned()), created_at: Some(created_at) })
}

pub fn read_import(path: &Path, current: &Snapshot) -> Result<(Snapshot, u64), String> {
    let metadata = fs::metadata(path).map_err(|_| "backup_read_failed".to_string())?;
    if metadata.len() > MAX_BACKUP_BYTES { return Err("backup_too_large".into()); }
    let bytes = fs::read(path).map_err(|_| "backup_read_failed".to_string())?;
    let document = parse_document_bytes(&bytes)?;
    let created_at = document.created_at;
    Ok((snapshot_from_document(document, current)?, created_at))
}

pub fn is_standard_backup_name(name: &str) -> bool {
    name.strip_prefix("desktop-dashboard-backup-").and_then(|value| value.strip_suffix(".json")).is_some_and(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn standard_backups(directory: &Path) -> Result<Vec<(SystemTime, PathBuf)>, String> {
    if !directory.exists() { return Ok(Vec::new()); }
    let mut files = Vec::new();
    for item in fs::read_dir(directory).map_err(|_| "backup_status_failed")? {
        let item = item.map_err(|_| "backup_status_failed")?;
        let name = item.file_name().to_string_lossy().into_owned();
        if is_standard_backup_name(&name) && item.file_type().map_err(|_| "backup_status_failed")?.is_file() {
            let modified = item.metadata().and_then(|value| value.modified()).map_err(|_| "backup_status_failed")?;
            files.push((modified, item.path()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Ok(files)
}

pub fn retain_standard_backups(directory: &Path, retained: usize) -> Result<(), String> {
    let files = standard_backups(directory)?;
    for (_, path) in files.iter().take(files.len().saturating_sub(retained)) { fs::remove_file(path).map_err(|_| "backup_cleanup_failed")?; }
    Ok(())
}

pub fn get_status(data_dir: &Path) -> Result<BackupStatus, String> {
    let newest = standard_backups(&backup_directory(data_dir))?.pop();
    Ok(match newest {
        Some((modified, path)) => BackupStatus {
            last_backup_at: Some(modified.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()),
            last_backup_path: Some(path.to_string_lossy().into_owned()),
        },
        None => BackupStatus { last_backup_at: None, last_backup_path: None },
    })
}

pub fn auto_backup_due_from_newest(newest: Option<SystemTime>, interval_days: u32, now: SystemTime) -> bool {
    newest.and_then(|time| now.duration_since(time).ok()).is_none_or(|elapsed| elapsed >= Duration::from_secs(interval_days as u64 * 86_400))
}

pub fn auto_backup_if_due(data_dir: &Path, snapshot: &Snapshot) -> Result<Option<BackupOperation>, String> {
    let newest = standard_backups(&backup_directory(data_dir))?.pop().map(|item| item.0);
    if auto_backup_due_from_newest(newest, snapshot.settings.backup_interval_days, SystemTime::now()) { write_standard_backup(data_dir, snapshot).map(Some) } else { Ok(None) }
}

#[cfg(windows)]
pub fn choose_json_path(save: bool, owner: isize) -> Result<Option<PathBuf>, String> {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut file = vec![0u16; 32_768];
    if save {
        let default = format!("desktop-dashboard-backup-{}.json", now_seconds()).encode_utf16().collect::<Vec<_>>();
        file[..default.len()].copy_from_slice(&default);
    }
    let filter = "JSON 文件 (*.json)\0*.json\0所有文件 (*.*)\0*.*\0\0".encode_utf16().collect::<Vec<_>>();
    let title = if save { "导出 Codex Beacon 备份" } else { "导入 Codex Beacon 备份" }.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let extension = "json\0".encode_utf16().collect::<Vec<_>>();
    let mut dialog = OPENFILENAMEW::default();
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = owner as _;
    dialog.lpstrFilter = filter.as_ptr(); dialog.nFilterIndex = 1;
    dialog.lpstrFile = file.as_mut_ptr(); dialog.nMaxFile = file.len() as u32;
    dialog.lpstrTitle = title.as_ptr(); dialog.lpstrDefExt = extension.as_ptr();
    dialog.Flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST | OFN_HIDEREADONLY };
    let accepted = unsafe { if save { GetSaveFileNameW(&mut dialog) } else { GetOpenFileNameW(&mut dialog) } } != 0;
    if !accepted {
        return if unsafe { CommDlgExtendedError() } == 0 { Ok(None) } else { Err("backup_dialog_failed".into()) };
    }
    let length = file.iter().position(|value| *value == 0).ok_or_else(|| "backup_dialog_failed".to_string())?;
    Ok(Some(PathBuf::from(String::from_utf16(&file[..length]).map_err(|_| "backup_dialog_failed")?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Snapshot;
    use serde_json::json;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temporary_directory(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("desktop-dashboard-backup-{name}-{nonce}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn remove_directory(path: &std::path::Path) { let _ = std::fs::remove_dir_all(path); }

    #[test]
    fn backup_round_trip_omits_revision_and_provider_data() {
        let mut current = Snapshot::default();
        crate::display_layout::activate(&mut current, &crate::display_layout::DisplayConfig {
            key: "backup-display".into(),
            work_area: crate::model::LayoutWorkArea { width: 1920.0, height: 1040.0 },
        });
        current.revision = 41;
        current.providers.codex.state = "ready".into();
        current.providers.codex.data = Some(json!({"secret":"omit"}));
        let document = BackupDocument::from_snapshot(&current, 123);
        let value = serde_json::to_value(&document).unwrap();
        assert_eq!(value["format"], BACKUP_FORMAT);
        assert_eq!(value["version"], 2);
        assert_eq!(value["createdAt"], 123);
        assert!(value["data"].get("revision").is_none());
        assert!(value["data"].get("providers").is_none());

        let bytes = serde_json::to_vec(&document).unwrap();
        let parsed = parse_document_bytes(&bytes).unwrap();
        let imported = snapshot_from_document(parsed, &current).unwrap();
        assert_eq!(imported.revision, 42);
        assert_eq!(imported.providers, current.providers);
        assert_eq!(imported.settings, current.settings);
        assert_eq!(imported.widgets, current.widgets);
        assert_eq!(imported.content, current.content);
        assert_eq!(imported.active_layout_key, current.active_layout_key);
        assert_eq!(imported.layout_profiles, current.layout_profiles);
    }

    #[test]
    fn backup_format_rejects_future_unknown_and_malformed_documents() {
        let document = BackupDocument::from_snapshot(&Snapshot::default(), 123);
        let mut future = serde_json::to_value(&document).unwrap();
        future["version"] = json!(3);
        assert_eq!(parse_document_bytes(&serde_json::to_vec(&future).unwrap()), Err("backup_invalid".into()));

        let mut legacy = serde_json::to_value(&document).unwrap();
        legacy["version"] = json!(1);
        legacy["data"].as_object_mut().unwrap().remove("activeLayoutKey");
        legacy["data"].as_object_mut().unwrap().remove("layoutProfiles");
        assert!(parse_document_bytes(&serde_json::to_vec(&legacy).unwrap()).is_ok());

        let mut unknown = serde_json::to_value(&document).unwrap();
        unknown["unexpected"] = json!(true);
        assert_eq!(parse_document_bytes(&serde_json::to_vec(&unknown).unwrap()), Err("backup_invalid".into()));
        assert_eq!(parse_document_bytes(b"{"), Err("backup_invalid".into()));
    }

    #[test]
    fn imported_models_are_normalized_and_invalid_components_rejected() {
        let current = Snapshot::default();
        let mut document = BackupDocument::from_snapshot(&current, 123);
        document.data.settings.theme.accent = "aabbcc".into();
        let imported = snapshot_from_document(document, &current).unwrap();
        assert_eq!(imported.settings.theme.accent, "#AABBCC");

        let mut invalid = BackupDocument::from_snapshot(&current, 123);
        invalid.data.widgets.clear();
        assert_eq!(snapshot_from_document(invalid, &current), Err("backup_invalid".into()));
    }

    #[test]
    fn auto_backup_due_uses_inclusive_interval_boundary() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000_000);
        let interval = Duration::from_secs(7 * 24 * 60 * 60);
        assert!(!auto_backup_due_from_newest(Some(now - interval + Duration::from_secs(1)), 7, now));
        assert!(auto_backup_due_from_newest(Some(now - interval), 7, now));
        assert!(auto_backup_due_from_newest(None, 7, now));
    }

    #[test]
    fn retention_deletes_only_matching_standard_backups() {
        let directory = temporary_directory("retention");
        for index in 0..12 { std::fs::write(directory.join(format!("desktop-dashboard-backup-{index}.json")), b"{}").unwrap(); }
        for index in 0..2 { std::fs::write(directory.join(format!("user-export-{index}.json")), b"{}").unwrap(); }
        retain_standard_backups(&directory, 10).unwrap();
        let names = std::fs::read_dir(&directory).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(names.iter().filter(|name| is_standard_backup_name(name)).count(), 10);
        assert_eq!(names.iter().filter(|name| name.starts_with("user-export-")).count(), 2);
        remove_directory(&directory);
    }
}
