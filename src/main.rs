// KeyScroll - Rust core
// Phase 2.2: Smooth stop + generation counter for reliable re-press handling

#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static SCROLL_UP_ACTIVE: AtomicBool = AtomicBool::new(false);
static SCROLL_DOWN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SCROLL_UP_GEN: AtomicU32 = AtomicU32::new(0);
static SCROLL_DOWN_GEN: AtomicU32 = AtomicU32::new(0);

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
    // Stop opposite direction, increment generation for current direction
    if up {
        SCROLL_DOWN_ACTIVE.store(false, Ordering::SeqCst);
        SCROLL_DOWN_GEN.fetch_add(1, Ordering::SeqCst);
    } else {
        SCROLL_UP_ACTIVE.store(false, Ordering::SeqCst);
        SCROLL_UP_GEN.fetch_add(1, Ordering::SeqCst);
    }

    let gen_counter = if up { &SCROLL_UP_GEN } else { &SCROLL_DOWN_GEN };
    let active_flag = if up { &SCROLL_UP_ACTIVE } else { &SCROLL_DOWN_ACTIVE };
    let my_gen = gen_counter.fetch_add(1, Ordering::SeqCst) + 1;
    active_flag.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        unsafe { scroll_loop(up, my_gen) }
    });
}

unsafe fn scroll_loop(up: bool, my_gen: u32) {
    let key = if up { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let direction = if up { 1 } else { -1 };
    let gen = if up { &SCROLL_UP_GEN } else { &SCROLL_DOWN_GEN };
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

        send_wheel(delta * direction);
        std::thread::sleep(Duration::from_millis(interval));
    }

    // --- Smooth stop (100ms linear decay) ---
    for step in (1..=4).rev() {
        if gen.load(Ordering::SeqCst) != my_gen { return; }
        send_wheel(30 * step * direction);
        std::thread::sleep(Duration::from_millis(25));
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
