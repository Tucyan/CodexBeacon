use crate::{AppState,model::{Action,Snapshot},window};
use tauri::Manager;
use std::sync::atomic::Ordering;

pub async fn on_main<T:Send+'static>(app:&tauri::AppHandle,f:impl FnOnce(&tauri::AppHandle)->Result<T,String>+Send+'static)->Result<T,String>{
    let (tx,rx)=std::sync::mpsc::channel();let handle=app.clone();
    app.run_on_main_thread(move||{let _=tx.send(f(&handle));}).map_err(|_|"main_thread_unavailable")?;
    tauri::async_runtime::spawn_blocking(move||rx.recv_timeout(std::time::Duration::from_secs(10)).map_err(|_|"main_thread_timeout".to_string())?).await.map_err(|_|"main_thread_task".to_string())?
}
fn known(app:&tauri::AppHandle,label:&str)->Result<(),String>{
    if label=="settings" || crate::snapshot(app)?.widgets.iter().any(|w|label==format!("widget-{}",w.id)){Ok(())}else{Err("invalid_window".into())}
}
fn settings_only(window:&tauri::WebviewWindow,app:&tauri::AppHandle)->Result<(),String>{known(app,window.label())?;if window.label()=="settings"{Ok(())}else{Err("permission_denied".into())}}
#[tauri::command]
pub fn get_snapshot(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<Snapshot,String>{known(&app,window.label())?;Ok(crate::scoped(crate::snapshot(&app)?,window.label()))}

#[tauri::command]
pub fn get_backup_status(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<crate::backup::BackupStatus,String>{settings_only(&window,&app)?;crate::backup::get_status(&app.state::<AppState>().data_dir)}

#[tauri::command]
pub fn get_autostart_status(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<crate::autostart::AutostartStatus,String>{settings_only(&window,&app)?;crate::autostart::get_status()}

#[tauri::command]
pub fn set_autostart_enabled(window:tauri::WebviewWindow,app:tauri::AppHandle,enabled:bool)->Result<crate::autostart::AutostartStatus,String>{settings_only(&window,&app)?;crate::autostart::set_enabled(enabled)}

#[tauri::command]
pub async fn backup_now(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<crate::backup::BackupOperation,String>{
    settings_only(&window,&app)?;crate::lifecycle::flush_views(&app,None).await?;
    crate::backup::write_standard_backup(&app.state::<AppState>().data_dir,&crate::snapshot(&app)?)
}

async fn selected_backup_path(window:&tauri::WebviewWindow,save:bool)->Result<Option<std::path::PathBuf>,String>{
    let owner=window.hwnd().map_err(|_|"backup_dialog_failed")?.0 as isize;
    tauri::async_runtime::spawn_blocking(move||crate::backup::choose_json_path(save,owner)).await.map_err(|_|"backup_dialog_failed".to_string())?
}

#[tauri::command]
pub async fn export_backup(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<crate::backup::BackupOperation,String>{
    settings_only(&window,&app)?;let Some(path)=selected_backup_path(&window,true).await? else{return Ok(crate::backup::BackupOperation{status:"cancelled".into(),path:None,created_at:None})};
    crate::lifecycle::flush_views(&app,None).await?;crate::backup::export_to_path(&path,&crate::snapshot(&app)?)
}

#[tauri::command]
pub async fn import_backup(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<crate::backup::BackupOperation,String>{
    settings_only(&window,&app)?;let Some(path)=selected_backup_path(&window,false).await? else{return Ok(crate::backup::BackupOperation{status:"cancelled".into(),path:None,created_at:None})};
    crate::lifecycle::flush_views(&app,None).await?;
    let state=app.state::<AppState>();if state.stopping.load(Ordering::SeqCst){return Err("application_stopping".into());}
    state.writes_frozen.store(true,Ordering::SeqCst);
    let result:Result<(Snapshot,Snapshot,u64),String>=(||{
        let before=crate::snapshot(&app)?;let (candidate,created_at)=crate::backup::read_import(&path,&before)?;
        let mut store=state.store.lock().map_err(|_|"storage_lock")?;
        store.save(&candidate).map_err(|_|"database_save_failed".to_string())?;
        *state.snapshot.lock().map_err(|_|"state_lock")?=candidate.clone();
        *state.dirty.lock().map_err(|_|"dirty_lock")?=None;
        drop(store);crate::broadcast(&app,&candidate);
        Ok((before,candidate,created_at))
    })();
    state.writes_frozen.store(false,Ordering::SeqCst);
    let (before,candidate,created_at)=result?;
    let previous=before;let current=candidate.clone();let _=on_main(&app,move|handle|window::sync_windows(handle,Some(&previous),&current)).await;
    Ok(crate::backup::BackupOperation{status:"imported".into(),path:Some(path.to_string_lossy().into_owned()),created_at:Some(created_at)})
}

#[tauri::command]
pub async fn dispatch(window:tauri::WebviewWindow,app:tauri::AppHandle,action:Action)->Result<Snapshot,String>{
    let label=window.label().to_owned();known(&app,&label)?;
    // Validate the caller before asking any other renderer to flush.
    let mut probe=crate::snapshot(&app)?;crate::model::apply_action(&mut probe,&label,action.clone())?;
    let target=match &action {Action::Widget{id,enabled:Some(false),..}|Action::Hide{id}=>Some(format!("widget-{id}")),_=>None};
    if let Some(target)=target {if target!=label {crate::lifecycle::flush_views(&app,Some(&target)).await?;}}
    apply(&app,&label,action).await
}
pub async fn apply(app:&tauri::AppHandle,label:&str,action:Action)->Result<Snapshot,String>{
    let state=app.state::<AppState>();
    if state.stopping.load(Ordering::SeqCst){return Err("application_stopping".into());}
    if state.quitting.load(Ordering::SeqCst)&&!matches!(&action,Action::Content{..}){return Err("application_closing".into());}
    let reconcile=!matches!(&action,Action::Content{..}|Action::Retry{..});
    let retry=if let Action::Retry{provider}=&action{Some(provider.clone())}else{None};
    let (before,updated)={let mut value=state.snapshot.lock().map_err(|_|"state_lock")?;if state.writes_frozen.load(Ordering::SeqCst){return Err("application_closing".into());}let before=value.clone();crate::model::apply_action(&mut value,label,action)?;(before,value.clone())};
    crate::mark_dirty(&state);crate::broadcast(app,&updated);
    if let Some(provider)=retry {match provider.as_str(){"codex"=>state.retry_codex.store(true,Ordering::SeqCst),"reset"=>state.retry_reset.store(true,Ordering::SeqCst),_=>{}}}
    if reconcile {on_main(app,move|handle|{let current=crate::snapshot(handle)?;window::sync_windows(handle,Some(&before),&current)}).await?;}
    Ok(crate::scoped(updated,label))
}
#[tauri::command]
pub async fn widget_ready(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<(),String>{
    let label=window.label().to_owned();known(&app,&label)?;if label=="settings"{return Ok(());}
    on_main(&app,move|handle|{let snapshot=crate::snapshot(handle)?;let state=handle.state::<AppState>();let mut runtime=state.runtime.lock().map_err(|_|"runtime_lock")?;
        runtime.ready.insert(label.clone());
        if snapshot.widgets.iter().any(|w|label==format!("widget-{}",w.id)&&w.enabled&&w.visible_wanted){let above=snapshot.settings.show_desktop_mode=="auto"&&runtime.desktop==window::policy::Desktop::Active;runtime.request_show(handle,&label,above,false);}
        Ok(())}).await
}
#[tauri::command]
pub async fn appearance_ready(window:tauri::WebviewWindow,app:tauri::AppHandle,revision:u64)->Result<(),String>{
    let label=window.label().to_owned();known(&app,&label)?;
    on_main(&app,move|handle|{let snapshot=crate::snapshot(handle)?;handle.state::<AppState>().runtime.lock().map_err(|_|"runtime_lock")?.acknowledge(handle,&label,revision,&snapshot);Ok(())}).await
}
#[tauri::command]
pub fn flush_ready(window:tauri::WebviewWindow,app:tauri::AppHandle,revision:u64)->Result<(),String>{
    let state=app.state::<AppState>();let mut pending=state.flushes.lock().map_err(|_|"flush_lock")?;
    if let Some(waiter)=pending.get_mut(&revision){waiter.labels.remove(window.label());if waiter.labels.is_empty(){if let Some(done)=pending.remove(&revision){let _=done.done.send(());}}}Ok(())
}
fn layout_allowed(window:&tauri::WebviewWindow,app:&tauri::AppHandle)->Result<(),String>{
    let value=crate::snapshot(app)?;
    if value.settings.layout_locked || !value.widgets.iter().any(|w|window.label()==format!("widget-{}",w.id)&&w.enabled&&w.visible_wanted){Err("layout_locked_or_hidden".into())}else{Ok(())}
}
#[tauri::command]
pub fn start_drag(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<(),String>{layout_allowed(&window,&app)?;window.start_dragging().map_err(|_|"drag_failed".into())}
#[tauri::command]
pub fn start_resize(window:tauri::WebviewWindow,app:tauri::AppHandle,direction:String)->Result<(),String>{
    layout_allowed(&window,&app)?;
    use tauri_runtime::ResizeDirection::*;
    let direction=match direction.to_ascii_lowercase().as_str(){"north"|"top"=>North,"south"|"bottom"=>South,"east"|"right"=>East,"west"|"left"=>West,"north-east"|"top-right"|"topright"=>NorthEast,"north-west"|"top-left"|"topleft"=>NorthWest,"south-east"|"bottom-right"|"bottomright"=>SouthEast,"south-west"|"bottom-left"|"bottomleft"=>SouthWest,_=>return Err("invalid_resize_direction".into())};
    let webview:&tauri::Webview=window.as_ref();
    webview.window().start_resize_dragging(direction).map_err(|_|"resize_failed".into())
}
