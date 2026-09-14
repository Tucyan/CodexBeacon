mod model;
mod storage;
mod window;
mod providers;
mod commands;
mod lifecycle;
mod smoke;
mod backup;
mod autostart;

use model::{Snapshot,ProviderSnapshot};
use std::{collections::{HashMap,HashSet},fs::File,sync::{Mutex,atomic::{AtomicBool,Ordering}},time::Instant};
use tauri::{Manager,Emitter};

pub(crate) struct FlushWaiter { labels:HashSet<String>, done:std::sync::mpsc::Sender<()> }
pub struct AppState {
    smoke_audit:smoke::AuditState,
    snapshot:Mutex<Snapshot>, store:Mutex<storage::Store>,
    pub runtime:Mutex<window::Runtime>, pub applying_geometry:AtomicBool,
    pub stopping:AtomicBool, pub retry_codex:AtomicBool, pub retry_reset:AtomicBool,
    writes_frozen:AtomicBool,
    dirty:Mutex<Option<Instant>>, flushes:Mutex<HashMap<u64,FlushWaiter>>,
    flush_token:std::sync::atomic::AtomicU64, quitting:AtomicBool,
    workers:Mutex<Vec<std::thread::JoinHandle<()>>>, _instance_lock:File,
    data_dir:std::path::PathBuf,
}
static LOG_PATH:std::sync::OnceLock<std::path::PathBuf>=std::sync::OnceLock::new();
pub(crate) fn record(code:&str) {
    use std::io::Write;
    if let Some(path)=LOG_PATH.get(){if let Ok(mut file)=std::fs::OpenOptions::new().create(true).append(true).open(path){let _=writeln!(file,"{} {code}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());}}
}
fn snapshot(app:&tauri::AppHandle)->Result<Snapshot,String>{app.state::<AppState>().snapshot.lock().map(|s|s.clone()).map_err(|_|"state_lock".into())}
fn scoped(mut value:Snapshot,label:&str)->Snapshot {
    if let Some(id)=label.strip_prefix("widget-"){
        value.content.retain(|key,_|key==id);value.widgets.retain(|widget|widget.id==id);
        if id!="codex-usage"{value.providers.codex=Default::default();}
        if id!="codex-reset"{value.providers.reset=Default::default();}
    }
    value
}
fn broadcast(app:&tauri::AppHandle,value:&Snapshot){
    for label in app.webview_windows().into_keys(){let _=app.emit_to(label.as_str(),"dashboard-state",scoped(value.clone(),&label));}
}
fn mark_dirty(state:&AppState){if let Ok(mut dirty)=state.dirty.lock(){*dirty=Some(Instant::now());}}
pub(crate) fn publish_provider(app:&tauri::AppHandle,name:&str,value:ProviderSnapshot){
    let state=app.state::<AppState>();
    if state.stopping.load(Ordering::SeqCst){return;}
    let Ok(mut current)=state.snapshot.lock() else{return;};
    if state.writes_frozen.load(Ordering::SeqCst){return;}
    match name {"codex"=>current.providers.codex=value,"reset"=>current.providers.reset=value,_=>return}
    current.revision+=1;let updated=current.clone();drop(current);broadcast(app,&updated);
}
pub fn run(){
    let app=tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_snapshot,commands::dispatch,commands::get_backup_status,commands::get_autostart_status,commands::set_autostart_enabled,commands::backup_now,commands::export_backup,commands::import_backup,commands::start_drag,commands::start_resize,commands::widget_ready,commands::appearance_ready,commands::flush_ready,smoke::renderer_audit])
        .setup(|app|{
            use std::os::windows::fs::OpenOptionsExt;
            let data=std::env::var_os("DASHBOARD_DATA_DIR").map(std::path::PathBuf::from).unwrap_or(app.path().app_data_dir()?);
            std::fs::create_dir_all(&data)?;
            let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).share_mode(0).open(data.join("instance.lock"))?;
            let _=LOG_PATH.set(data.join("runtime.log"));
            let store=storage::Store::open(&data.join("dashboard.db"))?;
            let mut initial=store.load()?.unwrap_or_default();
            model::normalize_snapshot(&mut initial)?;
            app.manage(AppState{smoke_audit:Default::default(),snapshot:Mutex::new(initial.clone()),store:Mutex::new(store),runtime:Mutex::new(window::Runtime::new()),applying_geometry:AtomicBool::new(false),stopping:AtomicBool::new(false),writes_frozen:AtomicBool::new(false),retry_codex:AtomicBool::new(false),retry_reset:AtomicBool::new(false),dirty:Mutex::new(Some(Instant::now())),flushes:Mutex::new(HashMap::new()),flush_token:std::sync::atomic::AtomicU64::new(0),quitting:AtomicBool::new(false),workers:Mutex::new(Vec::new()),_instance_lock:lock,data_dir:data});
            lifecycle::tray(app.handle())?;
            window::sync_windows(app.handle(),None,&initial)?;
            lifecycle::start_workers(app.handle());
            if autostart::refresh_on_startup().is_err() { record("autostart_refresh_failed"); }
            record("application_started");
            Ok(())
        })
        .on_window_event(|window,event|lifecycle::window_event(window,event))
        .build(tauri::generate_context!());
    match app {
        Ok(app)=>app.run(|handle,event|{
            if let tauri::RunEvent::ExitRequested{api,..}=event {
                if !handle.state::<AppState>().stopping.load(Ordering::SeqCst){api.prevent_exit();}
            }
        }),
        Err(_)=>{record("application_start_failed");eprintln!("Desktop Dashboard could not start. Check runtime.log or an existing instance.");}
    }
}
