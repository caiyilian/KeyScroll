// KeyScroll — keyboard scroll via global hotkeys
// Phase 4.1: System tray icon with context menu (raw Win32 FFI)

#![windows_subsystem = "windows"]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

mod config;
mod log;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::env;
use std::path::PathBuf;

// Win32 types
type HWND = isize; type HINSTANCE = isize; type HICON = isize; type HMENU = isize;
type WPARAM = usize; type LPARAM = isize; type LRESULT = isize; type BOOL = i32;

#[repr(C)]
struct MSG { hwnd: HWND, message: u32, wParam: WPARAM, lParam: LPARAM, time: u32, pt: [i32; 2] }
#[repr(C)]
struct POINT { x: i32, y: i32 }
#[repr(C)]
struct WNDCLASSW {
    style: u32, lpfnWndProc: Option<unsafe extern "system" fn(HWND,u32,WPARAM,LPARAM)->LRESULT>,
    cbClsExtra: i32, cbWndExtra: i32, hInstance: HINSTANCE, hIcon: HICON, hCursor: HICON,
    hbrBackground: isize, lpszMenuName: *const u16, lpszClassName: *const u16,
}
#[repr(C)]
struct NOTIFYICONDATAW {
    cbSize: u32, hWnd: HWND, uID: u32, uFlags: u32, uCallbackMessage: u32, hIcon: HICON,
    szTip: [u16;128], dwState: u32, dwStateMask: u32, szInfo: [u16;256], szInfoTitle: [u16;64],
    dwInfoFlags: u32, guidItem: [u8;16], hBalloonIcon: HICON,
}

#[link(name = "user32")] #[link(name = "shell32")] #[link(name = "comctl32")] #[link(name = "advapi32")]
extern "system" {
    fn GetModuleHandleW(n: *const u16) -> HINSTANCE;
    fn RegisterClassW(w: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(a:u32,b:*const u16,c:*const u16,d:u32,x:i32,y:i32,w:i32,h:i32,
        p:HWND,m:isize,i:HINSTANCE,l:*const())->HWND;
    fn DefWindowProcW(h:HWND,m:u32,w:WPARAM,l:LPARAM)->LRESULT;
    fn PostQuitMessage(i:i32);
    fn ShowWindow(h:HWND,s:i32)->BOOL;
    fn GetMessageW(m:*mut MSG,h:HWND,a:u32,b:u32)->BOOL;
    fn TranslateMessage(m:*const MSG)->BOOL;
    fn DispatchMessageW(m:*const MSG)->LRESULT;
    fn LoadIconW(h:HINSTANCE,n:*const u16)->HICON;
    fn CreatePopupMenu()->HMENU;
    fn AppendMenuW(m:HMENU,f:u32,i:WPARAM,s:*const u16)->BOOL;
    fn TrackPopupMenu(m:HMENU,f:u32,x:i32,y:i32,r:i32,h:HWND,p:*const())->BOOL;
    fn DestroyMenu(m:HMENU)->BOOL;
    fn SetForegroundWindow(h:HWND)->BOOL;
    fn GetCursorPos(p:*mut POINT)->BOOL;
    fn Shell_NotifyIconW(m:u32,d:*const NOTIFYICONDATAW)->BOOL;
    fn RegisterHotKey(h:HWND,i:i32,f:u32,v:u32)->BOOL;
    fn UnregisterHotKey(h:HWND,i:i32)->BOOL;
    fn GetAsyncKeyState(v:i32)->i16;
    fn SendInput(c:u32,p:*const u32,s:i32)->u32;
    fn RegOpenKeyExW(h:isize,s:*const u16,o:u32,a:u32,r:*mut isize)->i32;
    fn RegSetValueExW(h:isize,n:*const u16,z:u32,t:u32,d:*const u16,c:u32)->i32;
    fn RegDeleteValueW(h:isize,n:*const u16)->i32;
    fn RegCloseKey(h:isize)->i32;
    fn GetModuleFileNameW(h:isize,b:*mut u16,c:u32)->u32;
    fn ShellExecuteW(h:HWND,o:*const u16,f:*const u16,p:*const u16,d:*const u16,s:i32)->isize;
    fn GetForegroundWindow()->HWND;
    fn GetClassNameW(h:HWND,b:*mut u16,c:i32)->i32;
    fn SetProcessDPIAware()->BOOL;
}

const WM_HOTKEY:u32=0x0312; const WM_TRAYICON:u32=0x8001; const WM_DESTROY:u32=2;
const WM_COMMAND:u32=0x0111; const WM_LBUTTONUP:u32=0x0202; const WM_RBUTTONUP:u32=0x0205;
const NIM_ADD:u32=0; const NIM_MODIFY:u32=1; const NIM_DELETE:u32=2;
const NIF_MESSAGE:u32=1; const NIF_ICON:u32=2; const NIF_TIP:u32=4;
const NIF_INFO:u32=16; const NIF_SHOWTIP:u32=64; const NIIF_INFO:u32=1;
const MF_STRING:u32=0; const MF_SEPARATOR:u32=0x0800; const MF_CHECKED:u32=0x0008;
const TPM_LEFTALIGN:u32=0; const TPM_BOTTOMALIGN:u32=0x0020; const TPM_RIGHTBUTTON:u32=0x0800;
const INPUT_MOUSE:u32=0; const MOUSEEVENTF_WHEEL:u32=0x0800; const MOUSEEVENTF_HWHEEL:u32=0x1000;
const ID_EDIT:usize=1001; const ID_RELOAD:usize=1002; const ID_TOGGLE:usize=1003; const ID_EXIT:usize=1004; const ID_OPEN_LOG:usize=1005; const ID_JUMP_MODE:usize=1006;
// Registry constants
const HKEY_CURRENT_USER:isize = -2147483647i64 as isize;
const KEY_SET_VALUE:u32=0x0002; const KEY_WRITE:u32=0x20006; const REG_SZ:u32=1; const ERROR_SUCCESS:i32=0;

static V_GEN: AtomicU32 = AtomicU32::new(0);
static H_GEN: AtomicU32 = AtomicU32::new(0);
static SCROLLING_V: AtomicBool = AtomicBool::new(false);
static SCROLLING_H: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);
static JUMP_MODE: AtomicBool = AtomicBool::new(false);
static CFG_PATH: Mutex<Option<String>> = Mutex::new(None);
static LOGGER: Mutex<Option<log::Logger>> = Mutex::new(None);
static LAST_TRIGGER: AtomicU32 = AtomicU32::new(0);
const MISFIRE_MS: u64 = 300;

fn w(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

unsafe fn exe_path() -> Vec<u16> {
    let mut buf = vec![0u16; 1024];
    let len = GetModuleFileNameW(0, buf.as_mut_ptr(), 1024);
    buf.truncate(len as usize);
    buf
}

unsafe fn get_foreground_class() -> String {
    let hwnd = GetForegroundWindow();
    if hwnd == 0 { return String::new(); }
    let mut buf = [0u16; 128];
    let len = GetClassNameW(hwnd, buf.as_mut_ptr(), 128);
    if len <= 0 { return String::new(); }
    String::from_utf16_lossy(&buf[..len as usize])
}

fn find_per_app_config(class: &str) -> Option<config::PerAppConfig> {
    let path_str = CFG_PATH.lock().ok()?.clone()?;
    let path = PathBuf::from(&path_str);
    if !path.exists() { return None; }
    let content = std::fs::read_to_string(path).ok()?;
    let cfg: config::Config = toml::from_str(&content).ok()?;
    cfg.per_app.into_iter().find(|p| class.contains(&p.window_class) || p.window_class.contains(class))
}

unsafe fn install_autostart() -> bool {
    let key = w(RUN_KEY);
    let mut hkey: isize = 0;
    let rc = RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_WRITE, &mut hkey);
    if rc != ERROR_SUCCESS { return false; }
    let path = exe_path();
    let app = w("KeyScroll");
    let r = RegSetValueExW(hkey, app.as_ptr(), 0, REG_SZ, path.as_ptr(), path.len() as u32 * 2);
    RegCloseKey(hkey);
    r == ERROR_SUCCESS
}

unsafe fn uninstall_autostart() -> bool {
    let key = w(RUN_KEY);
    let mut hkey: isize = 0;
    let rc = RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_SET_VALUE, &mut hkey);
    if rc != ERROR_SUCCESS { return false; }
    let app = w("KeyScroll");
    let r = RegDeleteValueW(hkey, app.as_ptr());
    RegCloseKey(hkey);
    r == ERROR_SUCCESS
}

fn log_msg(msg: &str) {
    if let Ok(l) = LOGGER.lock() {
        if let Some(ref logger) = *l {
            logger.event(msg);
        }
    }
}

fn main() {
    unsafe {
        SetProcessDPIAware();
        // Handle --install / --uninstall flags before entering GUI loop
        let args: Vec<String> = env::args().collect();
        if args.iter().any(|a| a == "--install" || a == "-i") {
            if install_autostart() {
                std::process::exit(0);
            } else {
                eprintln!("Failed to install auto-start (try running as admin?)");
                std::process::exit(1);
            }
        }
        if args.iter().any(|a| a == "--uninstall" || a == "-u") {
            uninstall_autostart();
            std::process::exit(0);
        }
        // Initialize logger next to exe
        let path_bytes = exe_path();
        let path_str = String::from_utf16_lossy(&path_bytes);
        let p = PathBuf::from(&path_str);
        let log_path = p.with_extension("log");
if let Ok(mut l) = LOGGER.lock() {
            *l = Some(log::Logger::new(log_path));
        }
        log_msg("KeyScroll started");
        let inst = GetModuleHandleW(std::ptr::null());
        let cls = w("KeyScrollWnd"); let ttl = w("KeyScroll");
        let wc = WNDCLASSW {
            style: 3, lpfnWndProc: Some(callback),
            cbClsExtra: 0, cbWndExtra: 0, hInstance: inst,
            hIcon: LoadIconW(0, 32512 as *const u16), hCursor: 0, hbrBackground: 0,
            lpszMenuName: std::ptr::null(), lpszClassName: cls.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(0,cls.as_ptr(),ttl.as_ptr(),0,0,0,0,0,0,0,inst,std::ptr::null());
        ShowWindow(hwnd, 0);

        // Default hotkeys (hardcoded for now, config parsing available via --config)
        RegisterHotKey(hwnd,1,2,0x26); // Ctrl+Up
        RegisterHotKey(hwnd,2,2,0x28); // Ctrl+Down
        RegisterHotKey(hwnd,3,6,0x26); // Ctrl+Shift+Up
        RegisterHotKey(hwnd,4,6,0x28); // Ctrl+Shift+Down

        // Tray icon
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd, uID: 1, uFlags: NIF_MESSAGE|NIF_ICON|NIF_TIP|NIF_SHOWTIP,
            uCallbackMessage: WM_TRAYICON, hIcon: LoadIconW(0,32512 as *const u16),
            szTip: [0;128], dwState:0, dwStateMask:0,
            szInfo: [0;256], szInfoTitle: [0;64], dwInfoFlags: 0, guidItem: [0;16], hBalloonIcon: 0,
        };
        let tip = w("KeyScroll - Keyboard Scroll");
        for i in 0..tip.len().min(128) { nid.szTip[i] = tip[i]; }
        Shell_NotifyIconW(NIM_ADD, &nid);
        // Load config
        let (cfg, cfg_path) = config::load_config();
        *CFG_PATH.lock().unwrap() = Some(cfg_path.to_str().unwrap_or("config.toml").to_string());
        log_msg(&format!("Config loaded with {} per-app rules", cfg.per_app.len()));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg,0,0,0) != 0 {
            if msg.message == WM_HOTKEY && !PAUSED.load(Ordering::SeqCst) {
                let fg = get_foreground_class();
                match msg.wParam {
                    1 | 2 | 3 | 4 => {
                        let h = hwnd;
                        let up = msg.wParam == 1 || msg.wParam == 3;
                        let horiz = msg.wParam == 3 || msg.wParam == 4;
                        let dir_label = if up { "Up" } else { "Down" };
                        let hz_label = if horiz { "Horiz" } else { "Vert" };
                        log_msg(&format!("Hotkey: {}/{} fg:{}", hz_label, dir_label, &fg));
                        if JUMP_MODE.load(Ordering::SeqCst) {
                            // Jump mode: anti-misfire prevents double-jumps from one tap
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u32)
                                .unwrap_or(0);
                            let last = LAST_TRIGGER.load(Ordering::SeqCst);
                            if (now as u64) < (last as u64) + MISFIRE_MS && last != 0 {
                                log_msg("Misfire suppressed");
                            } else {
                                LAST_TRIGGER.store(now, Ordering::SeqCst);
                                set_tray_tip(h, &format!("KeyScroll - Scrolling {}/{}", hz_label, dir_label));
                                std::thread::spawn(move||unsafe{ jump_scroll(up, horiz, h) });
                            }
                        } else {
                            // Hold-to-scroll: skip if already scrolling in this direction
                            let scrolling = if horiz { &SCROLLING_H } else { &SCROLLING_V };
                            if scrolling.load(Ordering::SeqCst) {
                                // Already scrolling — ignore repeat WM_HOTKEY
                            } else {
                                scrolling.store(true, Ordering::SeqCst);
                                let g = match msg.wParam { 1|2 => V_GEN.fetch_add(1,Ordering::SeqCst)+1, _ => H_GEN.fetch_add(1,Ordering::SeqCst)+1 };
                                let gen: &AtomicU32 = if msg.wParam < 3 { &V_GEN } else { &H_GEN };
                                set_tray_tip(h, &format!("KeyScroll - Scrolling {}/{}", hz_label, dir_label));
                                std::thread::spawn(move||unsafe{ scroll(up, g, gen, horiz, h) });
                            }
                        }
                    }
                    _ => {}
                }
            }
            TranslateMessage(&msg); DispatchMessageW(&msg);
        }
    }
}

unsafe fn set_tray_tip(h: HWND, txt: &str) {
    let wide = w(txt);
    let mut n = NOTIFYICONDATAW{
        cbSize:std::mem::size_of::<NOTIFYICONDATAW>()as u32,hWnd:h,uID:1,
        uFlags:NIF_TIP,..std::mem::zeroed()
    };
    for i in 0..wide.len().min(128){n.szTip[i]=wide[i];}
    Shell_NotifyIconW(NIM_MODIFY,&n);
}

unsafe extern "system" fn callback(h:HWND,m:u32,w:WPARAM,l:LPARAM)->LRESULT {
    match m {
        WM_DESTROY => { PostQuitMessage(0); 0 }
        WM_TRAYICON if l as u32 == WM_RBUTTONUP => { show_menu(h); 0 }
        WM_TRAYICON if l as u32 == WM_LBUTTONUP => { show_info(h); 0 }
        WM_COMMAND => { cmd(h,w as u32); 0 }
        _ => DefWindowProcW(h,m,w,l),
    }
}

unsafe fn show_menu(h:HWND) {
    let m = CreatePopupMenu(); if m==0 { return; }
    AppendMenuW(m,MF_STRING,ID_OPEN_LOG,w("Open Log").as_ptr());
    AppendMenuW(m,MF_SEPARATOR,0,std::ptr::null());
    AppendMenuW(m,MF_STRING,ID_EDIT,w("Edit Config").as_ptr());
    AppendMenuW(m,MF_STRING,ID_RELOAD,w("Reload Config").as_ptr());
    AppendMenuW(m,MF_SEPARATOR,0,std::ptr::null());
    let p = if PAUSED.load(Ordering::SeqCst) { "Resume" } else { "Pause" };
    AppendMenuW(m,MF_STRING,ID_TOGGLE,w(p).as_ptr());
    AppendMenuW(m,MF_SEPARATOR,0,std::ptr::null());
    let jf = MF_STRING | if JUMP_MODE.load(Ordering::SeqCst) { MF_CHECKED } else { 0 };
    AppendMenuW(m,jf,ID_JUMP_MODE,w("Jump Mode").as_ptr());
    AppendMenuW(m,MF_SEPARATOR,0,std::ptr::null());
    AppendMenuW(m,MF_STRING,ID_EXIT,w("Exit").as_ptr());
    SetForegroundWindow(h);
    let mut pt = POINT{x:0,y:0}; GetCursorPos(&mut pt);
    TrackPopupMenu(m,TPM_LEFTALIGN|TPM_BOTTOMALIGN|TPM_RIGHTBUTTON,pt.x,pt.y,0,h,std::ptr::null());
    DestroyMenu(m);
}

unsafe fn show_info(h:HWND) {
    let s = if PAUSED.load(Ordering::SeqCst){"Paused"}else{"Active"};
    let txt = w(&format!("KeyScroll - {}\r\nCtrl+Up/Down to scroll",s));
    let mut n = NOTIFYICONDATAW{
        cbSize:std::mem::size_of::<NOTIFYICONDATAW>()as u32,hWnd:h,uID:1,
        uFlags:NIF_INFO,dwInfoFlags:NIIF_INFO,..std::mem::zeroed()
    };
    let ti=w("KeyScroll"); for i in 0..ti.len().min(64){n.szInfoTitle[i]=ti[i];}
    for i in 0..txt.len().min(256){n.szInfo[i]=txt[i];}
    Shell_NotifyIconW(NIM_MODIFY,&n);
}

unsafe fn cmd(h:HWND,id:u32) {
    match id as usize {
        ID_OPEN_LOG => {
            let open = w("open");
            let log_path_str = exe_path();
            let log_path = {
                let s = String::from_utf16_lossy(&log_path_str);
                let p = PathBuf::from(&s);
                w(p.with_extension("log").to_str().unwrap_or("keyscroll.log"))
            };
            ShellExecuteW(0, open.as_ptr(), log_path.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
        }
        ID_TOGGLE => {
            PAUSED.fetch_xor(true,Ordering::SeqCst);
            let paused = PAUSED.load(Ordering::SeqCst);
            log_msg(if paused{"Paused"}else{"Resumed"});
            let txt = if paused{"KeyScroll - Paused"}else{"KeyScroll - Resumed"};
            let wide=w(txt);
            let mut n=NOTIFYICONDATAW{
                cbSize:std::mem::size_of::<NOTIFYICONDATAW>()as u32,hWnd:h,uID:1,
                uFlags:NIF_TIP,..std::mem::zeroed()
            };
            for i in 0..wide.len().min(128){n.szTip[i]=wide[i];}
            Shell_NotifyIconW(NIM_MODIFY,&n);
        }
        ID_JUMP_MODE => {
            JUMP_MODE.fetch_xor(true,Ordering::SeqCst);
            if JUMP_MODE.load(Ordering::SeqCst) { log_msg("Jump Mode ON"); } else { log_msg("Jump Mode OFF"); }
        }
        ID_EXIT => { log_msg("Shutdown"); PostQuitMessage(0); }
        _ => {}
    }
}

unsafe fn scroll(up:bool,my_gen:u32,gen:&AtomicU32,horiz:bool,h:HWND) {
    let flag: &AtomicBool = if horiz { &SCROLLING_H } else { &SCROLLING_V };
    let dir = if up { 1 } else { -1 };
    let start = Instant::now();
    loop {
        if gen.load(Ordering::SeqCst) != my_gen { flag.store(false, Ordering::SeqCst); set_tray_tip(h, "KeyScroll - Keyboard Scroll"); return; }
        if GetAsyncKeyState(if up{0x26}else{0x28}) >= 0 { break; }
        let e = start.elapsed().as_millis() as u64;
        let (d,iv) = if e<500{(120,80)}else if e<2000{(240,40)}else{(480,20)};
        let flags = if horiz{MOUSEEVENTF_HWHEEL}else{MOUSEEVENTF_WHEEL};
        let mut buf=[0u32;7]; buf[0]=0; buf[5]=(d*dir)as u32; buf[6]=flags;
        SendInput(1,&buf as *const u32,std::mem::size_of::<[u32;7]>()as i32);
        std::thread::sleep(Duration::from_millis(iv));
    }
    for s in (1..=4usize).rev() {
        if gen.load(Ordering::SeqCst) != my_gen { flag.store(false, Ordering::SeqCst); return; }
        let flags = if horiz{MOUSEEVENTF_HWHEEL}else{MOUSEEVENTF_WHEEL};
        let mut buf=[0u32;7]; buf[0]=0; buf[5]=(30*s as i32*dir)as u32; buf[6]=flags;
        SendInput(1,&buf as *const u32,std::mem::size_of::<[u32;7]>()as i32);
        std::thread::sleep(Duration::from_millis(25));
    }
    flag.store(false, Ordering::SeqCst);
    set_tray_tip(h, "KeyScroll - Keyboard Scroll");
}

unsafe fn jump_scroll(up:bool,horiz:bool,h:HWND) {
    let dir:i32 = if up { 1 } else { -1 };
    let flags = if horiz { MOUSEEVENTF_HWHEEL } else { MOUSEEVENTF_WHEEL };
    for _ in 0..5 {
        let mut buf=[0u32;7]; buf[0]=0; buf[5]=(120*dir)as u32; buf[6]=flags;
        SendInput(1,&buf as *const u32,std::mem::size_of::<[u32;7]>()as i32);
        std::thread::sleep(Duration::from_millis(50));
    }
    set_tray_tip(h, "KeyScroll - Keyboard Scroll");
}
