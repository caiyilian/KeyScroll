// KeyScroll - Rust core
// Phase 2.3: Horizontal scroll with Ctrl+Shift+Up/Down

#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Vertical scroll state
static V_ACTIVE: AtomicBool = AtomicBool::new(false);
static V_GEN: AtomicU32 = AtomicU32::new(0);

// Horizontal scroll state
static H_ACTIVE: AtomicBool = AtomicBool::new(false);
static H_GEN: AtomicU32 = AtomicU32::new(0);

fn main() {
    unsafe {
        RegisterHotKey(None, 1, MOD_CONTROL, VK_UP.0 as u32).unwrap();
        RegisterHotKey(None, 2, MOD_CONTROL, VK_DOWN.0 as u32).unwrap();
        RegisterHotKey(None, 3, MOD_CONTROL | MOD_SHIFT, VK_UP.0 as u32).unwrap();
        RegisterHotKey(None, 4, MOD_CONTROL | MOD_SHIFT, VK_DOWN.0 as u32).unwrap();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as u32 {
                    1 => toggle_v(true),
                    2 => toggle_v(false),
                    3 => toggle_h(true),  // Ctrl+Shift+Up = scroll left
                    4 => toggle_h(false), // Ctrl+Shift+Down = scroll right
                    _ => {}
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn toggle_v(up: bool) {
    // Clear opposite direction
    let key = if up { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let my_gen = V_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    V_ACTIVE.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        unsafe { scroll_loop(key, up, my_gen, &V_GEN, false) }
    });
}

fn toggle_h(left: bool) {
    let key = if left { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let my_gen = H_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    H_ACTIVE.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        unsafe { scroll_loop(key, left, my_gen, &H_GEN, true) }
    });
}

unsafe fn scroll_loop(
    key: i32,
    positive: bool,
    my_gen: u32,
    gen: &AtomicU32,
    horizontal: bool,
) {
    let direction = if positive { 1 } else { -1 };
    let start = Instant::now();

    // --- Active scrolling ---
    loop {
        if gen.load(Ordering::SeqCst) != my_gen { return; }
        if GetAsyncKeyState(key) >= 0 { break; }

        let elapsed = start.elapsed();
        let (delta, interval) = if elapsed < Duration::from_millis(500) {
            (120, 80)
        } else if elapsed < Duration::from_millis(2000) {
            (240, 40)
        } else {
            (480, 20)
        };

        send(delta * direction, horizontal);
        std::thread::sleep(Duration::from_millis(interval));
    }

    // --- Smooth stop ---
    for step in (1..=4).rev() {
        if gen.load(Ordering::SeqCst) != my_gen { return; }
        send(30 * step * direction, horizontal);
        std::thread::sleep(Duration::from_millis(25));
    }
}

unsafe fn send(delta: i32, horizontal: bool) {
    let flags = if horizontal { MOUSEEVENTF_HWHEEL } else { MOUSEEVENTF_WHEEL };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0, dy: 0,
                mouseData: delta as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}
