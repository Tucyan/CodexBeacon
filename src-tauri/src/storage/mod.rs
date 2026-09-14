use crate::model::Snapshot;
use serde_json::{Map, Value};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;

mod sqlite {
    use super::{c_char, c_int, c_void};

    #[repr(C)] pub struct Db { _private: [u8; 0] }
    #[repr(C)] pub struct Stmt { _private: [u8; 0] }
    pub const OK: c_int = 0;
    pub const ROW: c_int = 100;
    pub const DONE: c_int = 101;
    pub const OPEN_READWRITE: c_int = 0x0000_0002;
    pub const OPEN_CREATE: c_int = 0x0000_0004;
    pub const OPEN_FULLMUTEX: c_int = 0x0001_0000;
    pub const TRANSIENT: *mut c_void = -1isize as *mut c_void;

    #[link(name = "winsqlite3")]
    extern "C" {
        pub fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut Db, flags: c_int, vfs: *const c_char) -> c_int;
        pub fn sqlite3_close(db: *mut Db) -> c_int;
        pub fn sqlite3_errmsg(db: *mut Db) -> *const c_char;
        pub fn sqlite3_exec(db: *mut Db, sql: *const c_char, callback: Option<unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int>, arg: *mut c_void, errmsg: *mut *mut c_char) -> c_int;
        pub fn sqlite3_free(ptr: *mut c_void);
        pub fn sqlite3_prepare_v2(db: *mut Db, sql: *const c_char, bytes: c_int, stmt: *mut *mut Stmt, tail: *mut *const c_char) -> c_int;
        pub fn sqlite3_bind_text(stmt: *mut Stmt, index: c_int, value: *const c_char, bytes: c_int, destructor: *mut c_void) -> c_int;
        pub fn sqlite3_step(stmt: *mut Stmt) -> c_int;
        pub fn sqlite3_finalize(stmt: *mut Stmt) -> c_int;
        pub fn sqlite3_column_text(stmt: *mut Stmt, column: c_int) -> *const u8;
        pub fn sqlite3_column_bytes(stmt: *mut Stmt, column: c_int) -> c_int;
        pub fn sqlite3_column_int(stmt: *mut Stmt, column: c_int) -> c_int;
    }
}

const CURRENT_SCHEMA_VERSION: i32 = 1;
const MIGRATION_1: &str = include_str!("../../migrations/001_initial.sql");

pub struct Store { db: *mut sqlite::Db }

// Store is moved into the root's single Mutex and only called through that owner.
// The raw SQLite connection must never be shared by separate owners.
unsafe impl Send for Store {}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let filename = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "database path contains NUL".to_string())?;
        let mut db = ptr::null_mut();
        let rc = unsafe { sqlite::sqlite3_open_v2(filename.as_ptr(), &mut db, sqlite::OPEN_READWRITE | sqlite::OPEN_CREATE | sqlite::OPEN_FULLMUTEX, ptr::null()) };
        if rc != sqlite::OK {
            let error = if db.is_null() { format!("sqlite open failed ({rc})") } else { sqlite_error(db, rc) };
            if !db.is_null() { unsafe { sqlite::sqlite3_close(db); } }
            return Err(error);
        }
        let mut store = Self { db };
        if let Err(error) = store.initialize() {
            drop(store);
            return Err(error);
        }
        Ok(store)
    }

    pub fn load(&self) -> Result<Option<Snapshot>, String> {
        let revision = match self.query_one("SELECT value_json FROM settings WHERE key = 'revision'")? {
            Some(value) => value.parse::<u64>().map_err(|_| "stored revision is invalid".to_string())?,
            None => return Ok(None),
        };
        let settings_text = self.query_one("SELECT value_json FROM settings WHERE key = 'settings'")?.ok_or_else(|| "stored settings are missing".to_string())?;
        let settings: Value = serde_json::from_str(&settings_text).map_err(|e| format!("stored settings JSON is invalid: {e}"))?;
        let mut widgets = Vec::new();
        {
            let mut stmt = Statement::prepare(self.db, "SELECT id, type, enabled, visible_wanted, x, y, width, height, scale, scale_mode, config_json FROM widget_instances ORDER BY rowid")?;
            loop {
                match stmt.step()? {
                    sqlite::DONE => break,
                    sqlite::ROW => {
                        let config_text = stmt.text(10)?;
                        let config = serde_json::from_str(&config_text).map_err(|e| format!("stored widget config JSON is invalid: {e}"))?;
                        let mut widget = Map::new();
                        widget.insert("id".into(), Value::String(stmt.text(0)?));
                        widget.insert("type".into(), Value::String(stmt.text(1)?));
                        widget.insert("enabled".into(), Value::Bool(stmt.int(2) != 0));
                        widget.insert("visibleWanted".into(), Value::Bool(stmt.int(3) != 0));
                        for (column, field) in [(4, "x"), (5, "y"), (6, "width"), (7, "height"), (8, "scale")] {
                            let number = stmt.text(column)?.parse::<f64>().map_err(|_| format!("stored widget {field} is invalid"))?;
                            widget.insert(field.into(), number_value(number)?);
                        }
                        widget.insert("scaleMode".into(), Value::String(stmt.text(9)?));
                        widget.insert("config".into(), config);
                        widgets.push(Value::Object(widget));
                    }
                    other => return Err(format!("sqlite step failed ({other})")),
                }
            }
        }
        let mut content = Map::new();
        for (widget_id, memo, todos, countdowns) in self.read_content_rows()? {
            let mut value = Map::new();
            value.insert("memo".into(), Value::String(memo));
            value.insert("todos".into(), todos);
            value.insert("countdowns".into(), countdowns);
            content.insert(widget_id, Value::Object(value));
        }
        let mut snapshot = serde_json::to_value(Snapshot::default()).map_err(|e| format!("default snapshot serialization failed: {e}"))?;
        snapshot["revision"] = Value::Number(serde_json::Number::from(revision));
        snapshot["settings"] = settings;
        snapshot["widgets"] = Value::Array(widgets);
        snapshot["content"] = Value::Object(content);
        serde_json::from_value(snapshot).map(Some).map_err(|e| format!("stored snapshot is invalid: {e}"))
    }

    pub fn save(&mut self, snapshot: &Snapshot) -> Result<(), String> {
        let value = serde_json::to_value(snapshot).map_err(|e| format!("snapshot serialization failed: {e}"))?;
        let settings = value.get("settings").cloned().ok_or_else(|| "snapshot settings are missing".to_string())?;
        let widgets = value.get("widgets").and_then(Value::as_array).ok_or_else(|| "snapshot widgets are missing".to_string())?;
        let content = value.get("content").and_then(Value::as_object).ok_or_else(|| "snapshot content is missing".to_string())?;
        let revision = value.get("revision").and_then(Value::as_u64).ok_or_else(|| "snapshot revision is invalid".to_string())?;
        let mut transaction = Transaction::begin(self.db)?;
        let result = (|| {
            transaction.exec("DELETE FROM memo; DELETE FROM todos; DELETE FROM countdowns; DELETE FROM widget_instances; DELETE FROM settings;")?;
            self.insert_text("INSERT INTO settings(key, value_json) VALUES (?, ?)", &["revision", &revision.to_string()])?;
            let settings_json = serde_json::to_string(&settings).map_err(|e| format!("settings serialization failed: {e}"))?;
            self.insert_text("INSERT INTO settings(key, value_json) VALUES (?, ?)", &["settings", &settings_json])?;
            for widget in widgets {
                self.insert_widget(widget)?;
            }
            for (widget_id, widget_content) in content {
                self.insert_content(widget_id, widget_content)?;
            }
            transaction.commit()
        })();
        if let Err(error) = result {
            let rollback = transaction.rollback();
            return Err(match rollback { Ok(()) => error, Err(rollback_error) => format!("{error}; rollback failed: {rollback_error}") });
        }
        Ok(())
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.exec("PRAGMA foreign_keys = ON;")?;
        self.exec("PRAGMA journal_mode = WAL;")?;
        let version = self.user_version()?;
        if version > CURRENT_SCHEMA_VERSION { return Err(format!("future schema version {version} is not supported")); }
        if version == 0 {
            let mut transaction = Transaction::begin(self.db)?;
            let result = (|| { transaction.exec(MIGRATION_1)?; transaction.exec("PRAGMA user_version = 1;")?; transaction.commit() })();
            if let Err(error) = result {
                let rollback = transaction.rollback();
                return Err(match rollback { Ok(()) => error, Err(rollback_error) => format!("{error}; rollback failed: {rollback_error}") });
            }
        }
        Ok(())
    }

    fn exec(&self, sql: &str) -> Result<(), String> { exec_raw(self.db, sql) }

    fn user_version(&self) -> Result<i32, String> {
        let mut statement = Statement::prepare(self.db, "PRAGMA user_version")?;
        if statement.step()? != sqlite::ROW { return Err("sqlite user_version returned no row".to_string()); }
        Ok(statement.int(0))
    }

    fn query_one(&self, sql: &str) -> Result<Option<String>, String> {
        let mut statement = Statement::prepare(self.db, sql)?;
        match statement.step()? { sqlite::ROW => Ok(Some(statement.text(0)?)), sqlite::DONE => Ok(None), rc => Err(format!("sqlite step failed ({rc})")) }
    }

    fn insert_text(&self, sql: &str, values: &[&str]) -> Result<(), String> {
        let mut statement = Statement::prepare(self.db, sql)?;
        for (index, value) in values.iter().enumerate() { statement.bind_text(index as i32 + 1, value)?; }
        statement.expect_done()
    }

    fn insert_widget(&self, widget: &Value) -> Result<(), String> {
        let object = widget.as_object().ok_or_else(|| "widget is not an object".to_string())?;
        let id = required_string(object, "id")?;
        let kind = required_string(object, "type")?;
        let enabled = required_bool(object, "enabled")?;
        let visible_wanted = required_bool(object, "visibleWanted")?;
        let x = required_number(object, "x")?;
        let y = required_number(object, "y")?;
        let width = required_number(object, "width")?;
        let height = required_number(object, "height")?;
        let scale = required_number(object, "scale")?;
        let scale_mode = required_string(object, "scaleMode")?;
        let config = object.get("config").ok_or_else(|| "widget config is missing".to_string())?;
        let config_json = serde_json::to_string(config).map_err(|e| format!("widget config serialization failed: {e}"))?;
        let values = [id, kind, bool_text(enabled), bool_text(visible_wanted), &x.to_string(), &y.to_string(), &width.to_string(), &height.to_string(), &scale.to_string(), scale_mode, &config_json];
        self.insert_text("INSERT INTO widget_instances(id,type,enabled,visible_wanted,x,y,width,height,scale,scale_mode,config_json) VALUES (?,?,?,?,?,?,?,?,?,?,?)", &values)
    }

    fn insert_content(&self, widget_id: &str, content: &Value) -> Result<(), String> {
        let object = content.as_object().ok_or_else(|| format!("content for {widget_id} is not an object"))?;
        let memo = required_string(object, "memo")?;
        self.insert_text("INSERT INTO memo(widget_id,text) VALUES (?,?)", &[widget_id, memo])?;
        let todos = object.get("todos").and_then(Value::as_array).ok_or_else(|| format!("todos for {widget_id} are missing"))?;
        for (position, todo) in todos.iter().enumerate() {
            let item = todo.as_object().ok_or_else(|| "todo is not an object".to_string())?;
            let id = required_string(item, "id")?;
            let text = required_string(item, "text")?;
            let completed = required_bool(item, "completed")?;
            self.insert_text("INSERT INTO todos(widget_id,id,position,text,completed) VALUES (?,?,?,?,?)", &[widget_id, id, &position.to_string(), text, bool_text(completed)])?;
        }
        let countdowns = object.get("countdowns").and_then(Value::as_array).ok_or_else(|| format!("countdowns for {widget_id} are missing"))?;
        for (position, countdown) in countdowns.iter().enumerate() {
            let item = countdown.as_object().ok_or_else(|| "countdown is not an object".to_string())?;
            let id = required_string(item, "id")?;
            let name = required_string(item, "name")?;
            let target = required_number(item, "target")?;
            self.insert_text("INSERT INTO countdowns(widget_id,id,position,name,target) VALUES (?,?,?,?,?)", &[widget_id, id, &position.to_string(), name, &target.to_string()])?;
        }
        Ok(())
    }

    fn read_content_rows(&self) -> Result<Vec<(String, String, Value, Value)>, String> {
        let mut ids = Vec::new();
        let mut widgets = Statement::prepare(self.db, "SELECT id FROM widget_instances ORDER BY rowid")?;
        while widgets.step()? == sqlite::ROW { ids.push(widgets.text(0)?); }
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let memo = self.query_one_bound("SELECT text FROM memo WHERE widget_id = ?", &id)?.unwrap_or_default();
            let mut todos = Vec::new();
            let mut stmt = Statement::prepare(self.db, "SELECT id,text,completed FROM todos WHERE widget_id = ? ORDER BY position")?;
            stmt.bind_text(1, &id)?;
            while stmt.step()? == sqlite::ROW { todos.push(serde_json::json!({"id":stmt.text(0)?,"text":stmt.text(1)?,"completed":stmt.int(2) != 0})); }
            let mut countdowns = Vec::new();
            let mut stmt = Statement::prepare(self.db, "SELECT id,name,target FROM countdowns WHERE widget_id = ? ORDER BY position")?;
            stmt.bind_text(1, &id)?;
            while stmt.step()? == sqlite::ROW {
                let target = stmt.text(2)?.parse::<f64>().map_err(|_| "stored countdown target is invalid".to_string())?;
                countdowns.push(serde_json::json!({"id":stmt.text(0)?,"name":stmt.text(1)?,"target":number_value(target)?}));
            }
            result.push((id, memo, Value::Array(todos), Value::Array(countdowns)));
        }
        Ok(result)
    }

    fn query_one_bound(&self, sql: &str, value: &str) -> Result<Option<String>, String> {
        let mut statement = Statement::prepare(self.db, sql)?;
        statement.bind_text(1, value)?;
        match statement.step()? { sqlite::ROW => Ok(Some(statement.text(0)?)), sqlite::DONE => Ok(None), rc => Err(format!("sqlite step failed ({rc})")) }
    }

    #[cfg(test)]
    pub(crate) fn table_exists_for_test(&self, table: &str) -> bool {
        let mut statement = Statement::prepare(self.db, "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?").expect("test statement prepares");
        statement.bind_text(1, table).expect("test bind succeeds");
        statement.step().expect("test query succeeds") == sqlite::ROW
    }
}

impl Drop for Store { fn drop(&mut self) { if !self.db.is_null() { unsafe { let _ = sqlite::sqlite3_close(self.db); } self.db = ptr::null_mut(); } } }

struct Transaction { db: *mut sqlite::Db, done: bool }
impl Transaction {
    fn begin(db: *mut sqlite::Db) -> Result<Self, String> { exec_raw(db, "BEGIN IMMEDIATE;")?; Ok(Self { db, done: false }) }
    fn exec(&self, sql: &str) -> Result<(), String> { exec_raw(self.db, sql) }
    fn commit(&mut self) -> Result<(), String> { exec_raw(self.db, "COMMIT;")?; self.done = true; Ok(()) }
    fn rollback(&mut self) -> Result<(), String> { if self.done { return Ok(()); } exec_raw(self.db, "ROLLBACK;")?; self.done = true; Ok(()) }
}
impl Drop for Transaction { fn drop(&mut self) { if !self.done { unsafe { let _ = sqlite::sqlite3_exec(self.db, c_string("ROLLBACK;").as_ptr(), None, ptr::null_mut(), ptr::null_mut()); } } } }

struct Statement { db: *mut sqlite::Db, stmt: *mut sqlite::Stmt }
impl Statement {
    fn prepare(db: *mut sqlite::Db, sql: &str) -> Result<Self, String> {
        let sql = c_string(sql);
        let mut stmt = ptr::null_mut();
        let rc = unsafe { sqlite::sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != sqlite::OK { return Err(sqlite_error(db, rc)); }
        Ok(Self { db, stmt })
    }
    fn bind_text(&mut self, index: i32, value: &str) -> Result<(), String> {
        if value.len() > c_int::MAX as usize { return Err("text value is too large".to_string()); }
        // sqlite3_bind_text receives an explicit byte length, so embedded NUL bytes
        // are valid payload. SQLITE_TRANSIENT makes SQLite copy before this borrow ends.
        let rc = unsafe { sqlite::sqlite3_bind_text(self.stmt, index, value.as_ptr().cast(), value.len() as c_int, sqlite::TRANSIENT) };
        if rc == sqlite::OK { Ok(()) } else { Err(sqlite_error(self.db, rc)) }
    }
    fn step(&mut self) -> Result<i32, String> { let rc = unsafe { sqlite::sqlite3_step(self.stmt) }; if rc == sqlite::ROW || rc == sqlite::DONE { Ok(rc) } else { Err(sqlite_error(self.db, rc)) } }
    fn expect_done(&mut self) -> Result<(), String> { if self.step()? == sqlite::DONE { Ok(()) } else { Err("sqlite statement unexpectedly returned a row".to_string()) } }
    fn text(&self, column: i32) -> Result<String, String> {
        let ptr = unsafe { sqlite::sqlite3_column_text(self.stmt, column) };
        if ptr.is_null() { return Ok(String::new()); }
        let length = unsafe { sqlite::sqlite3_column_bytes(self.stmt, column) };
        if length < 0 { return Err("sqlite returned invalid text length".to_string()); }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
        String::from_utf8(bytes.to_vec()).map_err(|_| "sqlite text is not valid UTF-8".to_string())
    }
    fn int(&self, column: i32) -> i32 { unsafe { sqlite::sqlite3_column_int(self.stmt, column) } }
}
impl Drop for Statement { fn drop(&mut self) { if !self.stmt.is_null() { unsafe { let _ = sqlite::sqlite3_finalize(self.stmt); } self.stmt = ptr::null_mut(); } } }

fn exec_raw(db: *mut sqlite::Db, sql: &str) -> Result<(), String> {
    let sql = c_string(sql);
    let mut message = ptr::null_mut();
    let rc = unsafe { sqlite::sqlite3_exec(db, sql.as_ptr(), None, ptr::null_mut(), &mut message) };
    if rc == sqlite::OK { return Ok(()); }
    let error = if !message.is_null() { unsafe { CStr::from_ptr(message).to_string_lossy().into_owned() } } else { sqlite_error(db, rc) };
    if !message.is_null() { unsafe { sqlite::sqlite3_free(message.cast()); } }
    Err(error)
}

fn sqlite_error(db: *mut sqlite::Db, rc: i32) -> String { unsafe { let message = sqlite::sqlite3_errmsg(db); if message.is_null() { format!("sqlite error ({rc})") } else { format!("sqlite error ({rc}): {}", CStr::from_ptr(message).to_string_lossy()) } } }
fn c_string(value: &str) -> CString { CString::new(value).expect("static SQL must not contain NUL") }
fn bool_text(value: bool) -> &'static str { if value { "1" } else { "0" } }
fn required_string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> { object.get(key).and_then(Value::as_str).ok_or_else(|| format!("snapshot field {key} is missing or invalid")) }
fn required_bool(object: &Map<String, Value>, key: &str) -> Result<bool, String> { object.get(key).and_then(Value::as_bool).ok_or_else(|| format!("snapshot field {key} is missing or invalid")) }
fn required_number(object: &Map<String, Value>, key: &str) -> Result<f64, String> { let number = object.get(key).and_then(Value::as_f64).ok_or_else(|| format!("snapshot field {key} is missing or invalid"))?; if number.is_finite() { Ok(number) } else { Err(format!("snapshot field {key} is not finite")) } }
fn number_value(value: f64) -> Result<Value, String> { serde_json::Number::from_f64(value).map(Value::Number).ok_or_else(|| "non-finite number in database".to_string()) }

#[cfg(test)]
mod tests;

#[cfg(test)]
mod legacy_compatibility_tests {
    use super::Store;
    use crate::model::Snapshot;
    use serde_json::Value;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_database() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("desktop-dashboard-legacy-{nonce}.sqlite"))
    }

    fn remove_database(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn legacy_settings_read_save_round_trip_preserves_layout_and_content() {
        let path = temporary_database();
        let mut store = Store::open(&path).expect("database opens");
        let mut baseline = Snapshot::default();
        baseline.revision = 17;
        baseline.widgets[0].x = 777.0;
        baseline.content.get_mut("memo").expect("memo content exists").memo = "legacy content".into();
        store.save(&baseline).expect("baseline saves");

        let mut legacy = serde_json::to_value(&baseline).expect("snapshot serializes");
        let theme = legacy["settings"]["theme"].as_object_mut().expect("theme is object");
        theme.remove("fieldBackground");
        theme.remove("mutedForeground");
        let legacy_settings = serde_json::to_string(&legacy["settings"]).expect("legacy settings serialize");
        store
            .insert_text(
                "UPDATE settings SET value_json = ? WHERE key = 'settings'",
                &[&legacy_settings],
            )
            .expect("legacy settings update succeeds");

        let loaded = store.load().expect("legacy snapshot loads").expect("snapshot exists");
        assert_eq!(loaded.revision, 17);
        assert_eq!(loaded.widgets[0].x, 777.0);
        assert_eq!(loaded.content["memo"].memo, "legacy content");
        assert_eq!(loaded.settings.theme.field_background, "#4A525A");
        assert_eq!(loaded.settings.theme.muted_foreground, "#9EA6AC");

        store.save(&loaded).expect("compatibility snapshot saves");
        drop(store);
        let store = Store::open(&path).expect("database reopens");
        let round_trip = store.load().expect("round trip snapshot loads").expect("snapshot exists");
        assert_eq!(round_trip.widgets[0].x, 777.0);
        assert_eq!(round_trip.content["memo"].memo, "legacy content");
        let stored_settings = store.query_one("SELECT value_json FROM settings WHERE key = 'settings'")
            .expect("stored settings query succeeds")
            .expect("stored settings exist");
        let stored_settings: Value = serde_json::from_str(&stored_settings).expect("stored settings JSON parses");
        assert_eq!(stored_settings["theme"]["fieldBackground"], "#4A525A");
        assert_eq!(stored_settings["theme"]["mutedForeground"], "#9EA6AC");
        drop(store);
        remove_database(&path);
    }
}
