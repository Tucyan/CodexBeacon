pub mod native;
pub mod policy;
use crate::{AppState, model::{Snapshot,WidgetInstance,render_scale}};
use std::{collections::{HashMap,HashSet},time::{Duration,Instant}};
use tauri::{Manager,Emitter,WebviewUrl,WebviewWindowBuilder};
use serde::Serialize;
use policy::{Desktop,Placement};

#[derive(Clone,Serialize)] pub struct Appearance { pub revision:u64, pub animate:bool }
struct Pending { token:u64, above:bool, started:Instant, animate:bool }
pub struct Runtime {
    probe:native::DesktopProbe,
    pub ready:HashSet<String>,
    pending:HashMap<String,Pending>,
    token:u64,
    pub desktop:Desktop,
    pub closing:bool,
    attempts:HashMap<String,u8>,
}
impl Runtime {
    pub fn new()->Self { Self {probe:native::DesktopProbe::new(),ready:HashSet::new(),pending:HashMap::new(),token:0,desktop:Desktop::Unknown,closing:false,attempts:HashMap::new()} }
    pub fn shutdown(&mut self){ self.closing=true;self.pending.clear();self.probe.close(); }
    fn cancel(&mut self,app:&tauri::AppHandle,label:&str) {
        self.pending.remove(label); self.token+=1;
        if app.get_webview_window(label).is_some() {let _=app.emit_to(label,"dashboard-appearance",Appearance{revision:self.token,animate:false});}
    }
    pub fn request_show(&mut self,app:&tauri::AppHandle,label:&str,above:bool,animate:bool){
        if self.closing || self.pending.contains_key(label) {return;}
        let Some(window)=app.get_webview_window(label) else{return;};
        if !self.ready.contains(label) {return;}
        self.token+=1;
        let token=self.token;
        if !animate {
            if let Ok(hwnd)=window.hwnd() {if !native::place(hwnd.0 as usize,above,true){crate::record("window_place_failed");}}
            let _=app.emit_to(label,"dashboard-appearance",Appearance{revision:token,animate:false});return;
        }
        self.pending.insert(label.into(),Pending{token,above,started:Instant::now(),animate});
        let _=app.emit_to(label,"dashboard-appearance-prepare",Appearance{revision:token,animate});
    }
    pub fn acknowledge(&mut self,app:&tauri::AppHandle,label:&str,token:u64,snapshot:&Snapshot){
        if self.closing || !self.pending.get(label).is_some_and(|p|p.token==token){return;}
        let Some(pending)=self.pending.remove(label) else{return;};
        if !eligible(snapshot,label) {self.cancel(app,label);return;}
        if let Some(window)=app.get_webview_window(label) {
            if let Ok(hwnd)=window.hwnd(){
                if !native::place(hwnd.0 as usize,pending.above,true){crate::record("window_place_failed");}
            }
            let _=app.emit_to(label,"dashboard-appearance",Appearance{revision:token,animate:pending.animate});
        }
    }
    pub fn tick(&mut self,app:&tauri::AppHandle,snapshot:&Snapshot){
        if self.closing{return;}
        let desktop=self.probe.observe();
        if desktop!=self.desktop {self.attempts.clear();}
        self.desktop=desktop;
        let auto=snapshot.settings.show_desktop_mode=="auto";
        for widget in &snapshot.widgets {
            let label=format!("widget-{}",widget.id);
            let Some(window)=app.get_webview_window(&label) else{continue;};
            let Ok(hwnd)=window.hwnd() else{continue;}; let hwnd=hwnd.0 as usize;
            if !widget.enabled || !widget.visible_wanted {
                native::hide(hwnd); if self.pending.contains_key(&label){self.cancel(app,&label);} continue;
            }
            if self.pending.get(&label).is_some_and(|p|p.above && (!auto || desktop!=Desktop::Active)) {
                self.cancel(app,&label);
                native::place(hwnd,false,true);
            }
            if self.pending.get(&label).is_some_and(|p|p.started.elapsed()>Duration::from_secs(2)) {
                self.cancel(app,&label); native::place(hwnd,auto&&desktop==Desktop::Active,true);crate::record("appearance_prepare_timeout");
            }
            if self.pending.contains_key(&label) || !self.ready.contains(&label) {continue;}
            match policy::decide(auto,true,native::visible(hwnd),native::topmost(hwnd),desktop) {
                Placement::Topmost=>{
                    let attempt=self.attempts.entry(label.clone()).or_default();
                    if *attempt<3 {*attempt+=1;self.request_show(app,&label,true,snapshot.settings.fade_enabled);}
                }
                Placement::Bottom=>{ self.cancel(app,&label);native::place(hwnd,false,true); }
                Placement::Hide=>{native::hide(hwnd);self.cancel(app,&label);}
                Placement::None=>{}
            }
        }
    }
}
fn eligible(snapshot:&Snapshot,label:&str)->bool { snapshot.widgets.iter().any(|w|label==format!("widget-{}",w.id)&&w.enabled&&w.visible_wanted) }
pub fn geometry(window:&tauri::WebviewWindow,widget:&WidgetInstance,snapshot:&Snapshot){
    let dpi=window.primary_monitor().ok().flatten().map(|monitor|monitor.scale_factor()).unwrap_or_else(||window.scale_factor().unwrap_or(1.0)); let scale=render_scale(widget,&snapshot.settings);
    let (left,top,right,bottom)=native::work_area();
    let _=window.set_min_size(Some(tauri::LogicalSize::new(120.0*scale,100.0*scale)));
    let width=(widget.width*scale*dpi).round().max(1.0) as u32;
    let height=(widget.height*scale*dpi).round().max(1.0) as u32;
    let x=(left as f64+widget.x*dpi).round().clamp(left as f64,(right-width as i32).max(left) as f64) as i32;
    let y=(top as f64+widget.y*dpi).round().clamp(top as f64,(bottom-height as i32).max(top) as f64) as i32;
    if window.outer_size().ok()!=Some(tauri::PhysicalSize::new(width,height)){ let _=window.set_size(tauri::PhysicalSize::new(width,height)); }
    if window.outer_position().ok()!=Some(tauri::PhysicalPosition::new(x,y)){let _=window.set_position(tauri::PhysicalPosition::new(x,y));}
    let _=window.set_resizable(!snapshot.settings.layout_locked);
}
pub fn sync_windows(app:&tauri::AppHandle,before:Option<&Snapshot>,snapshot:&Snapshot)->Result<(),String>{
    let state=app.state::<AppState>();
    state.applying_geometry.store(true,std::sync::atomic::Ordering::SeqCst);
    let result=(||{
        for widget in &snapshot.widgets {
            let label=format!("widget-{}",widget.id);
            if !widget.enabled {
                if let Some(window)=app.get_webview_window(&label){let _=window.destroy();}
                let mut runtime=state.runtime.lock().map_err(|_|"runtime_lock")?;
                runtime.ready.remove(&label);runtime.cancel(app,&label);continue;
            }
            let existing=app.get_webview_window(&label);
            let fresh=existing.is_none();
            let window=if let Some(window)=existing {window}else{
                let created=WebviewWindowBuilder::new(app,&label,WebviewUrl::App(format!("index.html?widget={}",widget.id).into()))
                    .initialization_script(if std::env::var_os("DASHBOARD_SMOKE_TEST").is_some(){"window.__DASHBOARD_SMOKE__=true;"}else{""})
                    .title(format!("Desktop Dashboard · {}",widget.widget_type))
                    .inner_size(widget.width,widget.height).min_inner_size(120.0,100.0)
                    .decorations(false).transparent(true).shadow(false).skip_taskbar(true)
                    .minimizable(false).maximizable(false).visible(false).focused(false).build().map_err(|_|"window_create_failed")?;
                if let Ok(hwnd)=created.hwnd(){native::configure_widget(hwnd.0 as usize);native::place(hwnd.0 as usize,false,false);}
                created
            };
            geometry(&window,widget,snapshot);
            let mut runtime=state.runtime.lock().map_err(|_|"runtime_lock")?;
            let old=before.and_then(|s|s.widgets.iter().find(|w|w.id==widget.id));
            let settings_changed=before.is_some_and(|s|s.settings.show_desktop_mode!=snapshot.settings.show_desktop_mode || s.settings.fade_enabled!=snapshot.settings.fade_enabled);
            if settings_changed {
                runtime.cancel(app,&label);runtime.attempts.remove(&label);
                if snapshot.settings.show_desktop_mode!="auto" {
                    if let Ok(hwnd)=window.hwnd(){native::place(hwnd.0 as usize,false,widget.visible_wanted);}
                }
            }
            if !widget.visible_wanted {
                if let Ok(hwnd)=window.hwnd(){native::hide(hwnd.0 as usize);}runtime.cancel(app,&label);
            } else if !fresh && old.is_some_and(|w|!w.visible_wanted || !w.enabled) {
                let above=snapshot.settings.show_desktop_mode=="auto"&&runtime.desktop==Desktop::Active;
                runtime.attempts.remove(&label);runtime.request_show(app,&label,above,snapshot.settings.fade_enabled);
            }
        }
        Ok(())
    })();
    state.applying_geometry.store(false,std::sync::atomic::Ordering::SeqCst);
    result
}
pub fn settings(app:&tauri::AppHandle)->Result<(),String>{
    if let Some(window)=app.get_webview_window("settings") {let _=window.show();let _=window.set_focus();return Ok(());}
    WebviewWindowBuilder::new(app,"settings",WebviewUrl::App("index.html?settings=1".into())).title("Desktop Dashboard · 设置").inner_size(760.0,680.0).min_inner_size(580.0,480.0).build().map_err(|_|"settings_create_failed".to_string())?;
    Ok(())
}
