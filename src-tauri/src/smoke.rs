//! Isolated native integration checks, enabled only by DASHBOARD_SMOKE_TEST.
//! Reports identifiers and revisions, never widget content or account data.
use std::{collections::HashMap,sync::Mutex,time::Duration};
use tauri::Manager;
use crate::AppState;

#[derive(Default)]
pub struct Audit { latest:HashMap<String,u64>, mismatch:bool }
pub type AuditState=Mutex<Audit>;

#[tauri::command]
pub fn renderer_audit(window:tauri::WebviewWindow,app:tauri::AppHandle,revision:u64,widget_ids:Vec<String>,rendered_id:Option<String>)->Result<(),String>{
    if std::env::var_os("DASHBOARD_SMOKE_TEST").is_none(){return Err("diagnostic_disabled".into());}
    let id=window.label().strip_prefix("widget-").ok_or("invalid_window")?;
    let state=app.state::<AppState>();let mut audit=state.smoke_audit.lock().map_err(|_|"audit_lock")?;
    if widget_ids!=[id] || rendered_id.as_deref()!=Some(id){audit.mismatch=true;crate::record("smoke_renderer_identity_mismatch");}
    audit.latest.entry(window.label().into()).and_modify(|r|*r=(*r).max(revision)).or_insert(revision);
    Ok(())
}
pub fn start(app:&tauri::AppHandle){
    let handle=app.clone();
    std::thread::spawn(move||{
        std::thread::sleep(Duration::from_secs(4));
        // Exercise the actual native move/resize event path and repeat the
        // same scoped broadcasts used by the live providers.
        for step in 0..12 {
            let app=handle.clone();let _=handle.run_on_main_thread(move||{
                if let Some(window)=app.get_webview_window("widget-memo"){
                    let _=window.set_position(tauri::PhysicalPosition::new(64+step*3,72+step*2));
                    let _=window.set_size(tauri::PhysicalSize::new(352+step as u32*2,256+step as u32));
                }
                if let Ok(mut value)=app.state::<AppState>().snapshot.lock(){
                    value.revision+=1;let next=value.clone();drop(value);crate::broadcast(&app,&next);
                }
            });
            std::thread::sleep(Duration::from_millis(150));
        }
        std::thread::sleep(Duration::from_secs(2));
        let app=handle.clone();let _=handle.run_on_main_thread(move||{
            let state=app.state::<AppState>();
            let ready=state.runtime.lock().map(|r|r.ready.len()).unwrap_or(0);
            let handles=app.webview_windows().into_iter().filter(|(label,_)|label.starts_with("widget-")).filter_map(|(_,w)|w.hwnd().ok().map(|h|h.0 as usize)).collect::<std::collections::HashSet<_>>();
            let visible=handles.iter().all(|h|crate::window::native::visible(*h));
            crate::record(if ready==2&&handles.len()==2&&visible{"smoke_two_independent_renderers_passed"}else{"smoke_failed"});
            let revision=crate::snapshot(&app).map(|s|s.revision).unwrap_or(u64::MAX);
            let valid=state.smoke_audit.lock().map(|audit|!audit.mismatch&&["widget-memo","widget-todo"].iter().all(|label|audit.latest.get(*label)==Some(&revision))).unwrap_or(false);
            crate::record(if valid{"smoke_event_isolation_passed"}else{"smoke_event_isolation_failed"});
            crate::lifecycle::quit(app);
        });
    });
}
