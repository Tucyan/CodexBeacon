use crate::{AppState,model::Action};
use std::{sync::{atomic::Ordering,Arc},time::{Duration,Instant}};
use tauri::{Manager,Emitter};

pub async fn flush_views(app:&tauri::AppHandle,target:Option<&str>)->Result<(),String>{
    let state=app.state::<AppState>();
    let labels=app.webview_windows().into_keys().filter(|label|label.starts_with("widget-")&&target.is_none_or(|t|t==label.as_str())).collect::<std::collections::HashSet<_>>();
    if labels.is_empty(){return Ok(());}
    let token=state.flush_token.fetch_add(1,Ordering::SeqCst)+1;
    let (tx,rx)=std::sync::mpsc::channel();
    state.flushes.lock().map_err(|_|"flush_lock")?.insert(token,crate::FlushWaiter{labels:labels.clone(),done:tx});
    for label in labels {if app.get_webview_window(&label).is_some(){let _=app.emit_to(label.as_str(),"dashboard-flush",serde_json::json!({"revision":token}));}}
    let result=tauri::async_runtime::spawn_blocking(move||rx.recv_timeout(Duration::from_secs(5))).await;
    state.flushes.lock().map_err(|_|"flush_lock")?.remove(&token);
    match result {Ok(Ok(()))=>Ok(()),_=>{crate::record("renderer_flush_failed");Err("保存未完成，操作已取消。请重试保存后再退出或隐藏。".into())}}
}
fn persist(app:&tauri::AppHandle)->Result<(),String>{
    let state=app.state::<AppState>();
    let mut store=state.store.lock().map_err(|_|"storage_lock")?;
    let captured=Instant::now();let value=crate::snapshot(app)?;
    store.save(&value).map_err(|_|"database_save_failed".to_string())?;
    let mut dirty=state.dirty.lock().map_err(|_|"dirty_lock")?;
    if dirty.is_some_and(|changed|changed<=captured){*dirty=None;}
    Ok(())
}
fn alert(app:&tauri::AppHandle){
    crate::record("operation_cancelled_unsaved");
    let handle=app.clone();let _=app.run_on_main_thread(move||{
        let _=crate::window::settings(&handle);
        let text="保存尚未完成，操作已取消。请检查磁盘空间或重试后再退出。".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let title="Desktop Dashboard".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        unsafe{windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),windows_sys::Win32::UI::WindowsAndMessaging::MB_OK|windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONWARNING);}
    });
}
pub fn quit(app:tauri::AppHandle){
    if app.state::<AppState>().quitting.swap(true,Ordering::SeqCst){return;}
    tauri::async_runtime::spawn(async move{
        if flush_views(&app,None).await.is_err(){app.state::<AppState>().quitting.store(false,Ordering::SeqCst);alert(&app);return;}
        // Freeze under the same lock that serializes mutations. The final
        // SQLite transaction therefore includes every acknowledged edit.
        {let state=app.state::<AppState>();let Ok(_value)=state.snapshot.lock() else{alert(&app);return;};state.writes_frozen.store(true,Ordering::SeqCst);}
        if persist(&app).is_err(){let state=app.state::<AppState>();state.writes_frozen.store(false,Ordering::SeqCst);state.quitting.store(false,Ordering::SeqCst);alert(&app);return;}
        app.state::<AppState>().stopping.store(true,Ordering::SeqCst);
        let workers=app.state::<AppState>().workers.lock().map(|mut w|std::mem::take(&mut *w)).unwrap_or_default();
        let _=tauri::async_runtime::spawn_blocking(move||{for worker in workers{let _=worker.join();}}).await;
        let _=crate::commands::on_main(&app,|handle|{handle.state::<AppState>().runtime.lock().map_err(|_|"runtime_lock")?.shutdown();crate::record("application_exited_cleanly");handle.exit(0);Ok(())}).await;
    });
}
pub fn start_workers(app:&tauri::AppHandle){
    let handle=app.clone();let queued=Arc::new(std::sync::atomic::AtomicBool::new(false));
    let monitor=std::thread::spawn(move||{
        let mut save_warning=false;
        let mut last_backup_check=Instant::now().checked_sub(Duration::from_secs(3600)).unwrap_or_else(Instant::now);
        while !handle.state::<AppState>().stopping.load(Ordering::SeqCst){
            let dirty=handle.state::<AppState>().dirty.lock().ok().and_then(|d|*d).is_some_and(|d|d.elapsed()>=Duration::from_millis(400));
            if dirty {if persist(&handle).is_err(){if !save_warning{crate::record("database_save_failed");save_warning=true;}}else{save_warning=false;}}
            if last_backup_check.elapsed()>=Duration::from_secs(3600) {
                let clean=handle.state::<AppState>().dirty.lock().ok().is_some_and(|value|value.is_none());
                if clean {
                    last_backup_check=Instant::now();
                    if let Ok(snapshot)=crate::snapshot(&handle) {if crate::backup::auto_backup_if_due(&handle.state::<AppState>().data_dir,&snapshot).is_err(){crate::record("automatic_backup_failed");}}
                }
            }
            if !queued.swap(true,Ordering::SeqCst){let app=handle.clone();let flag=queued.clone();let _=handle.run_on_main_thread(move||{
                if let Ok(snapshot)=crate::snapshot(&app){if let Ok(mut runtime)=app.state::<AppState>().runtime.lock(){runtime.tick(&app,&snapshot);}}
                flag.store(false,Ordering::SeqCst);
            });}
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    let mut workers=vec![monitor];
    if std::env::var_os("DASHBOARD_SMOKE_TEST").is_none(){workers.extend(crate::providers::spawn(app.clone()));}
    *app.state::<AppState>().workers.lock().expect("worker state")=workers;
    if std::env::var_os("DASHBOARD_SMOKE_TEST").is_some(){
        crate::smoke::start(app);
    }
}
pub fn tray(app:&tauri::AppHandle)->Result<(),String>{
    use tauri::{menu::{Menu,MenuItem},tray::TrayIconBuilder};
    let show=MenuItem::with_id(app,"show","显示所有已启用组件",true,None::<&str>).map_err(|_|"tray_menu")?;
    let hide=MenuItem::with_id(app,"hide","隐藏所有组件",true,None::<&str>).map_err(|_|"tray_menu")?;
    let settings=MenuItem::with_id(app,"settings","设置",true,None::<&str>).map_err(|_|"tray_menu")?;
    let lock=MenuItem::with_id(app,"lock","切换布局锁定",true,None::<&str>).map_err(|_|"tray_menu")?;
    let quit_item=MenuItem::with_id(app,"quit","退出",true,None::<&str>).map_err(|_|"tray_menu")?;
    let menu=Menu::with_items(app,&[&show,&hide,&settings,&lock,&quit_item]).map_err(|_|"tray_menu")?;
    let mut rgba=vec![0u8;32*32*4];
    for y in 4..28 {for x in 4..28 {let i=(y*32+x)*4;rgba[i..i+4].copy_from_slice(if x<14||y<14 {&[130,221,184,255]}else{&[70,120,160,255]});}}
    TrayIconBuilder::with_id("dashboard-tray").icon(tauri::image::Image::new_owned(rgba,32,32)).tooltip("Desktop Dashboard").menu(&menu).show_menu_on_left_click(true).on_menu_event(|app,event|{
        let app=app.clone();let id=event.id().as_ref().to_owned();
        if id=="quit" {quit(app);return;}
        if id=="settings" {let _=crate::window::settings(&app);return;}
        tauri::async_runtime::spawn(async move{
            if id=="hide" && flush_views(&app,None).await.is_err(){alert(&app);return;}
            let Ok(snapshot)=crate::snapshot(&app) else{return;};
            if id=="lock" {let mut settings=snapshot.settings;settings.layout_locked=!settings.layout_locked;let _=crate::commands::apply(&app,"settings",Action::Settings{settings}).await;}
            else {for widget in snapshot.widgets {if widget.enabled {let action=if id=="show"{Action::Show{id:widget.id}}else{Action::Hide{id:widget.id}};let _=crate::commands::apply(&app,"settings",action).await;}}}
        });
    }).build(app).map_err(|_|"tray_create_failed".to_string())?;
    Ok(())
}
pub fn window_event(window:&tauri::Window,event:&tauri::WindowEvent){
    let label=window.label().to_owned();let app=window.app_handle().clone();
    if let tauri::WindowEvent::CloseRequested{api,..}=event {
        api.prevent_close();
        if label=="settings"{let _=window.hide();return;}
        if let Some(id)=label.strip_prefix("widget-").map(str::to_owned){tauri::async_runtime::spawn(async move{
            if flush_views(&app,Some(&label)).await.is_err(){alert(&app);return;}
            let _=crate::commands::apply(&app,"settings",Action::Hide{id}).await;
        });}
        return;
    }
    if !matches!(event,tauri::WindowEvent::Moved(_)|tauri::WindowEvent::Resized(_)){return;}
    let Some(id)=label.strip_prefix("widget-") else{return;};
    let state=app.state::<AppState>();
    if state.applying_geometry.load(Ordering::SeqCst){return;}
    let Ok(mut value)=state.snapshot.lock() else{return;};
    if state.writes_frozen.load(Ordering::SeqCst){return;}
    if value.settings.layout_locked {return;}
    let settings=value.settings.clone();
    let Some(widget)=value.widgets.iter_mut().find(|w|w.id==id) else{return;};
    if !widget.enabled||!widget.visible_wanted{return;}
    let Ok(hwnd)=window.hwnd() else{return;};
    // Minimized/hidden system geometry must never overwrite the saved layout.
    if !crate::window::native::visible(hwnd.0 as usize) || unsafe{windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd.0 as _)}!=0{return;}
    let Some((x,y,width,height))=crate::window::native::rect(hwnd.0 as usize) else{return;};
    if width<=0||height<=0{return;}
    let dpi=window.scale_factor().unwrap_or(1.0);let scale=crate::model::render_scale(widget,&settings);let (left,top,_,_)=crate::window::native::work_area();
    let next=((x-left) as f64/dpi,(y-top) as f64/dpi,width as f64/dpi/scale,height as f64/dpi/scale);
    if (widget.x-next.0).abs()<0.5&&(widget.y-next.1).abs()<0.5&&(widget.width-next.2).abs()<0.5&&(widget.height-next.3).abs()<0.5{return;}
    widget.x=next.0;widget.y=next.1;widget.width=next.2.max(120.0);widget.height=next.3.max(100.0);
    value.revision+=1;let snapshot=value.clone();drop(value);crate::mark_dirty(&state);crate::broadcast(&app,&snapshot);
}
