use serde::Serialize;
use std::path::{Path, PathBuf};

pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const VALUE_NAME: &str = "Desktop Dashboard";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub enabled: bool,
    pub path_current: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshDecision {
    SkipSmokeTest,
    KeepExisting,
    RefreshMissing,
}

pub fn quote_executable(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

pub fn parse_target_path(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if let Some(rest) = command.strip_prefix('"') {
        return rest.find('"').map(|end| PathBuf::from(&rest[..end]));
    }
    command.split_whitespace().next().map(PathBuf::from)
}

pub fn refresh_decision(
    existing_command: Option<&str>,
    current_exe: &Path,
    smoke_test: bool,
) -> RefreshDecision {
    if smoke_test {
        return RefreshDecision::SkipSmokeTest;
    }
    let Some(command) = existing_command else {
        return RefreshDecision::KeepExisting;
    };
    match parse_target_path(command) {
        Some(path) if normalize_path(&path) == normalize_path(current_exe) => RefreshDecision::KeepExisting,
        Some(path) if path.exists() => RefreshDecision::KeepExisting,
        _ => RefreshDecision::RefreshMissing,
    }
}

pub fn status_from_command(existing_command: Option<&str>, current_exe: &Path) -> AutostartStatus {
    let path_current = existing_command
        .and_then(parse_target_path)
        .is_some_and(|path| normalize_path(&path) == normalize_path(current_exe));
    AutostartStatus { enabled: existing_command.is_some(), path_current }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_ascii_lowercase()
}

fn current_executable() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|_| "autostart_exe_unavailable".into())
}

fn registry_value() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE};
        let subkey = wide(RUN_SUBKEY);
        let value_name = wide(VALUE_NAME);
        let mut key = std::ptr::null_mut();
        let result = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_QUERY_VALUE, &mut key) };
        if result == 2 {
            return Ok(None);
        }
        if result != 0 {
            return Err("autostart_registry_failed".into());
        }
        let mut value_type = 0;
        let mut size = 0u32;
        let result = unsafe { RegQueryValueExW(key, value_name.as_ptr(), std::ptr::null(), &mut value_type, std::ptr::null_mut(), &mut size) };
        if result == 2 {
            unsafe { RegCloseKey(key); }
            return Ok(None);
        }
        if result != 0 || value_type != 1 || size < 2 {
            unsafe { RegCloseKey(key); }
            return Err("autostart_registry_failed".into());
        }
        let mut bytes = vec![0u8; size as usize];
        let result = unsafe { RegQueryValueExW(key, value_name.as_ptr(), std::ptr::null(), &mut value_type, bytes.as_mut_ptr(), &mut size) };
        unsafe { RegCloseKey(key); }
        if result != 0 {
            return Err("autostart_registry_failed".into());
        }
        let words = bytes[..size as usize].chunks_exact(2).map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]])).take_while(|word| *word != 0).collect::<Vec<_>>();
        String::from_utf16(&words).map(Some).map_err(|_| "autostart_registry_failed".into())
    }
    #[cfg(not(windows))]
    {
        Err("autostart_unsupported".into())
    }
}

fn write_registry(command: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY_CURRENT_USER, REG_SZ};
        let subkey = wide(RUN_SUBKEY);
        let value_name = wide(VALUE_NAME);
        let mut key = std::ptr::null_mut();
        let result = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey.as_ptr(), &mut key) };
        if result != 0 {
            return Err("autostart_registry_failed".into());
        }
        let data = wide(command);
        let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
        let result = unsafe { RegSetValueExW(key, value_name.as_ptr(), 0, REG_SZ, bytes.as_ptr(), bytes.len() as u32) };
        unsafe { RegCloseKey(key); }
        if result == 0 { Ok(()) } else { Err("autostart_registry_failed".into()) }
    }
    #[cfg(not(windows))]
    {
        let _ = command;
        Err("autostart_unsupported".into())
    }
}

fn delete_registry() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegDeleteValueW, HKEY_CURRENT_USER, KEY_SET_VALUE};
        let subkey = wide(RUN_SUBKEY);
        let value_name = wide(VALUE_NAME);
        let mut key = std::ptr::null_mut();
        let result = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_SET_VALUE, &mut key) };
        if result == 2 { return Ok(()); }
        if result != 0 { return Err("autostart_registry_failed".into()); }
        let result = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
        unsafe { RegCloseKey(key); }
        if result == 0 || result == 2 { Ok(()) } else { Err("autostart_registry_failed".into()) }
    }
    #[cfg(not(windows))]
    {
        Err("autostart_unsupported".into())
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn get_status() -> Result<AutostartStatus, String> {
    let current = current_executable()?;
    Ok(status_from_command(registry_value()?.as_deref(), &current))
}

pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    if enabled {
        let current = current_executable()?;
        write_registry(&quote_executable(&current))?;
    } else {
        delete_registry()?;
    }
    get_status()
}

pub fn refresh_on_startup() -> Result<(), String> {
    if std::env::var_os("DASHBOARD_SMOKE_TEST").is_some() {
        return Ok(());
    }
    let current = current_executable()?;
    let existing = registry_value()?;
    if refresh_decision(existing.as_deref(), &current, false) == RefreshDecision::RefreshMissing {
        write_registry(&quote_executable(&current))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_executable_as_a_single_quoted_command_token() {
        assert_eq!(quote_executable(Path::new(r"C:\Program Files\Desktop Dashboard\dashboard.exe")), r#""C:\Program Files\Desktop Dashboard\dashboard.exe""#);
    }

    #[test]
    fn parses_quoted_and_unquoted_executable_paths_without_arguments() {
        assert_eq!(parse_target_path(r#""C:\Program Files\Desktop Dashboard\dashboard.exe" --startup"#), Some(PathBuf::from(r"C:\Program Files\Desktop Dashboard\dashboard.exe")));
        assert_eq!(parse_target_path(r"C:\Dashboard\dashboard.exe --startup"), Some(PathBuf::from(r"C:\Dashboard\dashboard.exe")));
        assert_eq!(parse_target_path("  "), None);
    }

    #[test]
    fn refreshes_only_missing_entries_and_never_in_smoke_test() {
        let current = std::env::current_exe().unwrap();
        assert_eq!(refresh_decision(None, &current, false), RefreshDecision::KeepExisting);
        assert_eq!(refresh_decision(Some(r#""C:\Missing\dashboard.exe""#), &current, false), RefreshDecision::RefreshMissing);
        let valid = quote_executable(&current);
        assert_eq!(refresh_decision(Some(&valid), &current, false), RefreshDecision::KeepExisting);
        assert_eq!(refresh_decision(Some(r#""C:\Missing\dashboard.exe""#), &current, true), RefreshDecision::SkipSmokeTest);
    }

    #[test]
    fn reports_registry_presence_and_current_path_without_exposing_path() {
        let current = Path::new(r"C:\Dashboard\dashboard.exe");
        assert_eq!(status_from_command(None, current), AutostartStatus { enabled: false, path_current: false });
        assert_eq!(status_from_command(Some(r#""C:\Dashboard\dashboard.exe""#), current), AutostartStatus { enabled: true, path_current: true });
        assert_eq!(status_from_command(Some(r#""C:\Old\dashboard.exe""#), current), AutostartStatus { enabled: true, path_current: false });
    }
}
