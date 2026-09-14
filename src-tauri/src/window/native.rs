use super::policy::Desktop;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use std::collections::HashSet;

fn wide(text:&str)->Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
fn handle(value:usize)->HWND { value as HWND }
const FLAGS:u32 = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;

pub struct DesktopProbe { sentinel:usize, baseline:bool, warned:bool }
impl DesktopProbe {
    pub fn new()->Self {
        // Built-in STATIC class: our own hidden reference window, never an
        // Explorer child. All callers run this on the application UI thread.
        let sentinel=unsafe { CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,wide("STATIC").as_ptr(),wide("DesktopDashboard.Reference").as_ptr(),WS_POPUP|WS_DISABLED,0,0,0,0,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null()) };
        if !sentinel.is_null() { unsafe { SetWindowPos(sentinel,HWND_BOTTOM,0,0,0,0,FLAGS); } }
        let mut probe=Self { sentinel:sentinel as usize, baseline:false, warned:false };
        probe.baseline=probe.ranks().is_some_and(|(host,reference)| host>reference);
        probe
    }
    fn ranks(&self)->Option<(usize,usize)> {
        if self.sentinel==0 { return None; }
        let order=window_order();
        let shell=unsafe {GetShellWindow()};
        let mut shell_pid=0;
        unsafe {GetWindowThreadProcessId(shell,&mut shell_pid);}
        let mut host=0usize;
        if !shell.is_null() && !unsafe {FindWindowExW(shell,std::ptr::null_mut(),wide("SHELLDLL_DefView").as_ptr(),std::ptr::null())}.is_null() { host=shell as usize; }
        if host==0 {
            for &hwnd in &order {
                if !visible(hwnd) || class_name(hwnd)!="WorkerW" { continue; }
                let mut pid=0; unsafe {GetWindowThreadProcessId(handle(hwnd),&mut pid);}
                if pid!=shell_pid { continue; }
                if !unsafe {FindWindowExW(handle(hwnd),std::ptr::null_mut(),wide("SHELLDLL_DefView").as_ptr(),std::ptr::null())}.is_null() { host=hwnd;break; }
            }
        }
        Some((order.iter().position(|h|*h==host)?,order.iter().position(|h|*h==self.sentinel)?))
    }
    pub fn observe(&mut self)->Desktop {
        let ranks=self.ranks();
        // Establish a baseline only when the desktop is actually below our
        // reference; never infer an active desktop from an unknown baseline.
        if !self.baseline {self.baseline=ranks.is_some_and(|(host,reference)|host>reference);}
        if !self.baseline || ranks.is_none() {
            if !self.warned {crate::record("desktop_probe_unknown");self.warned=true;}
            return Desktop::Unknown;
        }
        self.warned=false;
        match ranks { Some((host,reference)) if host<reference=>Desktop::Active,Some(_)=>Desktop::Inactive,None=>Desktop::Unknown }
    }
    pub fn close(&mut self) { if self.sentinel!=0 { unsafe {DestroyWindow(handle(self.sentinel));} self.sentinel=0; } }
}
pub fn window_order()->Vec<usize> {
    let mut result=Vec::new(); let mut seen=HashSet::new();
    let mut hwnd=unsafe {GetTopWindow(std::ptr::null_mut())};
    while !hwnd.is_null() && result.len()<4096 && seen.insert(hwnd as usize) {
        result.push(hwnd as usize); hwnd=unsafe {GetWindow(hwnd,GW_HWNDNEXT)};
    }
    result
}
fn class_name(hwnd:usize)->String { let mut text=[0u16;128]; let count=unsafe {GetClassNameW(handle(hwnd),text.as_mut_ptr(),128)}; if count>0 {String::from_utf16_lossy(&text[..count as usize])}else{String::new()} }
pub fn visible(hwnd:usize)->bool { unsafe { IsWindowVisible(handle(hwnd))!=0 } }
pub fn topmost(hwnd:usize)->bool { unsafe { GetWindowLongPtrW(handle(hwnd),GWL_EXSTYLE)&WS_EX_TOPMOST as isize!=0 } }
pub fn configure_widget(hwnd:usize) {
    unsafe {
        let style=GetWindowLongPtrW(handle(hwnd),GWL_EXSTYLE);
        SetWindowLongPtrW(handle(hwnd),GWL_EXSTYLE,(style|WS_EX_TOOLWINDOW as isize)&!(WS_EX_APPWINDOW as isize));
    }
}
pub fn hide(hwnd:usize) { unsafe { ShowWindow(handle(hwnd),SW_HIDE); } }
pub fn place(hwnd:usize,above:bool,show:bool)->bool {
    unsafe {
        if !above && SetWindowPos(handle(hwnd),HWND_NOTOPMOST,0,0,0,0,FLAGS)==0 { return false; }
        let flags=FLAGS | if show { SWP_SHOWWINDOW } else {0};
        SetWindowPos(handle(hwnd),if above {HWND_TOPMOST}else{HWND_BOTTOM},0,0,0,0,flags)!=0
    }
}
pub fn work_area()->(i32,i32,i32,i32) {
    let mut rect=windows_sys::Win32::Foundation::RECT {left:0,top:0,right:1920,bottom:1080};
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA,0,&mut rect as *mut _ as *mut _,0); }
    (rect.left,rect.top,rect.right,rect.bottom)
}
pub fn rect(hwnd:usize)->Option<(i32,i32,i32,i32)> {
    let mut value=windows_sys::Win32::Foundation::RECT::default();
    if unsafe {GetWindowRect(handle(hwnd),&mut value)}==0 {None}else{Some((value.left,value.top,value.right-value.left,value.bottom-value.top))}
}
