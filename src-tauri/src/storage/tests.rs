use super::{sqlite, Store};
use crate::model::Snapshot;
use serde_json::{json, Value};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_database(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("desktop-dashboard-storage-{name}-{nonce}.sqlite"))
}

fn with_snapshot_value<F>(snapshot: Snapshot, edit: F) -> Snapshot
where
    F: FnOnce(&mut Value),
{
    let mut value = serde_json::to_value(snapshot).expect("snapshot serializes");
    edit(&mut value);
    serde_json::from_value(value).expect("edited snapshot remains valid")
}

fn remove_database(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
}

#[test]
fn open_empty_database_has_no_snapshot_and_creates_business_tables() {
    let path = temporary_database("empty");
    let store = Store::open(&path).expect("database opens");
    assert!(store.load().expect("empty database loads").is_none());
    for table in ["settings", "widget_instances", "memo", "todos", "countdowns"] {
        assert!(store.table_exists_for_test(table), "missing table {table}");
    }
    drop(store);
    remove_database(&path);
}

#[test]
fn save_load_round_trip_preserves_unicode_and_omits_provider_data() {
    let path = temporary_database("unicode");
    let mut store = Store::open(&path).expect("database opens");
    let snapshot = with_snapshot_value(Snapshot::default(), |value| {
        value["revision"] = json!(42);
        value["settings"]["theme"]["accent"] = json!("#雪🌙");
        value["content"]["memo"]["memo"] = json!("中文内容\0memo – café");
        value["content"]["todo"]["todos"] = json!([
            {"id": "nul-todo", "text": "待办\0内容", "completed": false}
        ]);
        value["providers"]["codex"]["state"] = json!("ready");
        value["providers"]["codex"]["data"] = json!({"secret": "must not persist"});
    });
    let mut snapshot = snapshot;
    crate::display_layout::activate(&mut snapshot, &crate::display_layout::DisplayConfig {
        key: "test-display".into(),
        work_area: crate::model::LayoutWorkArea { width: 1920.0, height: 1040.0 },
    });
    store.save(&snapshot).expect("snapshot saves");
    drop(store);

    let store = Store::open(&path).expect("database reopens");
    let loaded = store.load().expect("snapshot loads").expect("snapshot exists");
    let loaded_value = serde_json::to_value(loaded).expect("loaded snapshot serializes");
    assert_eq!(loaded_value["revision"], json!(42));
    assert_eq!(loaded_value["settings"]["theme"]["accent"], json!("#雪🌙"));
    assert_eq!(loaded_value["content"]["memo"]["memo"], json!("中文内容\0memo – café"));
    assert_eq!(loaded_value["content"]["todo"]["todos"][0]["text"], json!("待办\0内容"));
    assert_eq!(loaded_value["providers"]["codex"]["state"], json!("idle"));
    assert_eq!(loaded_value["providers"]["codex"]["data"], Value::Null);
    assert_eq!(loaded_value["activeLayoutKey"], json!("test-display"));
    let profile_x = loaded_value["layoutProfiles"]["test-display"]["widgets"]["memo"]["x"].as_f64().unwrap();
    assert!((profile_x - 64.0).abs() < 0.001);
    drop(store);
    remove_database(&path);
}

#[test]
fn future_schema_version_is_rejected() {
    let path = temporary_database("future");
    create_database_with_user_version(&path, 99);
    let error = match Store::open(&path) {
        Ok(_) => panic!("future schema must be rejected"),
        Err(error) => error,
    };
    assert!(error.contains("future") || error.contains("schema"));
    remove_database(&path);
}

#[test]
fn duplicate_todo_id_rolls_back_every_change_in_save_transaction() {
    let path = temporary_database("rollback");
    let mut store = Store::open(&path).expect("database opens");
    let baseline = with_snapshot_value(Snapshot::default(), |value| {
        value["revision"] = json!(7);
        value["content"]["todo"]["todos"] = json!([
            {"id": "baseline", "text": "保留", "completed": false}
        ]);
    });
    store.save(&baseline).expect("baseline saves");
    let duplicate = with_snapshot_value(Snapshot::default(), |value| {
        value["revision"] = json!(8);
        value["content"]["todo"]["todos"] = json!([
            {"id": "same", "text": "first", "completed": false},
            {"id": "same", "text": "second", "completed": true}
        ]);
    });
    assert!(store.save(&duplicate).is_err());
    let loaded = store.load().expect("database remains readable").expect("baseline remains");
    let loaded_value = serde_json::to_value(loaded).expect("loaded snapshot serializes");
    assert_eq!(loaded_value["revision"], json!(7));
    assert_eq!(loaded_value["content"]["todo"]["todos"][0]["id"], json!("baseline"));
    drop(store);
    remove_database(&path);
}

#[cfg(windows)]
fn create_database_with_user_version(path: &Path, version: i32) {
    unsafe {
        let mut db: *mut sqlite::Db = ptr::null_mut();
        let filename = CString::new(path.to_string_lossy().as_bytes()).expect("path has no NUL");
        assert_eq!(sqlite::sqlite3_open_v2(filename.as_ptr(), &mut db, sqlite::OPEN_READWRITE | sqlite::OPEN_CREATE, ptr::null()), 0);
        let sql = CString::new(format!("PRAGMA user_version={version}")).expect("sql has no NUL");
        assert_eq!(sqlite::sqlite3_exec(db, sql.as_ptr(), None, ptr::null_mut(), ptr::null_mut()), 0);
        assert_eq!(sqlite::sqlite3_close(db), 0);
    }
}

#[cfg(not(windows))]
fn create_database_with_user_version(_path: &Path, _version: i32) {
    panic!("these storage bindings are Windows-only");
}
