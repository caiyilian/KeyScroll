// KeyScroll - Rust core
// Phase 2.1: Acceleration curve — scroll speed increases with hold duration.
// Uses RegisterHotKey + thread-local message loop.

#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static SCROLL_UP: AtomicBool = AtomicBool::new(false);
static SCROLL_DOWN: AtomicBool = AtomicBool::new(false);

fn main() {
    unsafe {
        RegisterHotKey(None, 1, MOD_CONTROL, VK_UP.0 as u32).unwrap();
        RegisterHotKey(None, 2, MOD_CONTROL, VK_DOWN.0 as u32).unwrap();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as u32 {
                    1 => toggle_scroll(true),
                    2 => toggle_scroll(false),
                    _ => {}
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn toggle_scroll(up: bool) {
    if up {
        SCROLL_DOWN.store(false, Ordering::SeqCst);
        if SCROLL_UP.swap(true, Ordering::SeqCst) { return; }
    } else {
        SCROLL_UP.store(false, Ordering::SeqCst);
        if SCROLL_DOWN.swap(true, Ordering::SeqCst) { return; }
    }

    std::thread::spawn(move || {
        let active = if up { &SCROLL_UP } else { &SCROLL_DOWN };
        unsafe { scroll_loop(up, active) }
    });
}

unsafe fn scroll_loop(up: bool, active: &AtomicBool) {
    let key = if up { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let direction = if up { 1 } else { -1 };
    let start = Instant::now();

    loop {
        if !active.load(Ordering::SeqCst) { break; }
        if GetAsyncKeyState(key) >= 0 {
            active.store(false, Ordering::SeqCst);
            break;
        }

        let elapsed = start.elapsed();
        let (delta, interval) = if elapsed < Duration::from_millis(500) {
            (120, 80)       // slow: 1 notch every 80ms
        } else if elapsed < Duration::from_millis(2000) {
            (240, 40)       // medium: 2 notches every 40ms
        } else {
            (480, 20)       // fast: 4 notches every 20ms
        };

        send_wheel(delta * direction);
        std::thread::sleep(Duration::from_millis(interval));
    }
}

unsafe fn send_wheel(delta: i32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0, dy: 0,
                mouseData: delta as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}
