// KeyScroll - Rust core
// Phase 3: Config-driven hotkeys and scroll behavior via TOML.

#![windows_subsystem = "windows"]

mod config;

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Scroll state
static V_GEN: AtomicU32 = AtomicU32::new(0);
static H_GEN: AtomicU32 = AtomicU32::new(0);

// Scroll parameters (copied from config, thread-safe Copy)
#[derive(Debug, Clone, Copy)]
struct ScrollParams {
    initial_interval_ms: u64,
    min_interval_ms: u64,
    accel_start_ms: u64,
    accel_max_ms: u64,
    step_size: u32,
    smooth_stop_ms: u64,
}

fn main() {
    let (cfg, _cfg_path) = config::load_config();

    // Parse hotkey bindings or fall back to defaults
    let bind_up = config::parse_hotkey(&cfg.hotkeys.scroll_up).unwrap_or(config::HotkeyBinding {
        modifiers: MOD_CONTROL,
        vk: VK_UP.0 as u32,
    });
    let bind_down = config::parse_hotkey(&cfg.hotkeys.scroll_down).unwrap_or(config::HotkeyBinding {
        modifiers: MOD_CONTROL,
        vk: VK_DOWN.0 as u32,
    });
    let bind_left = config::parse_hotkey(&cfg.hotkeys.scroll_left).unwrap_or(config::HotkeyBinding {
        modifiers: MOD_CONTROL | MOD_SHIFT,
        vk: VK_UP.0 as u32,
    });
    let bind_right = config::parse_hotkey(&cfg.hotkeys.scroll_right).unwrap_or(config::HotkeyBinding {
        modifiers: MOD_CONTROL | MOD_SHIFT,
        vk: VK_DOWN.0 as u32,
    });

    let params = ScrollParams {
        initial_interval_ms: cfg.scroll.initial_interval_ms,
        min_interval_ms: cfg.scroll.min_interval_ms,
        accel_start_ms: cfg.scroll.acceleration_start_ms,
        accel_max_ms: cfg.scroll.acceleration_max_ms,
        step_size: cfg.scroll.step_size,
        smooth_stop_ms: cfg.behavior.smooth_stop_ms,
    };

    unsafe {
        RegisterHotKey(None, 1, bind_up.modifiers, bind_up.vk).unwrap();
        RegisterHotKey(None, 2, bind_down.modifiers, bind_down.vk).unwrap();
        RegisterHotKey(None, 3, bind_left.modifiers, bind_left.vk).unwrap();
        RegisterHotKey(None, 4, bind_right.modifiers, bind_right.vk).unwrap();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as u32 {
                    1 => toggle_v(true, params),
                    2 => toggle_v(false, params),
                    3 => toggle_h(true, params),
                    4 => toggle_h(false, params),
                    _ => {}
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn toggle_v(up: bool, params: ScrollParams) {
    let key = if up { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let my_gen = V_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        unsafe { scroll_loop(key, up, my_gen, &V_GEN, false, params) }
    });
}

fn toggle_h(left: bool, params: ScrollParams) {
    let key = if left { VK_UP.0 as i32 } else { VK_DOWN.0 as i32 };
    let my_gen = H_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        unsafe { scroll_loop(key, left, my_gen, &H_GEN, true, params) }
    });
}

unsafe fn scroll_loop(
    key: i32,
    positive: bool,
    my_gen: u32,
    gen: &AtomicU32,
    horizontal: bool,
    params: ScrollParams,
) {
    let direction = if positive { 1 } else { -1 };
    let step = params.step_size as i32;
    let start = Instant::now();

    // --- Active scrolling ---
    loop {
        if gen.load(Ordering::SeqCst) != my_gen { return; }
        if GetAsyncKeyState(key) >= 0 { break; }

        let elapsed = start.elapsed().as_millis() as u64;

        // Compute delta and interval along config-driven acceleration curve
        let (delta, interval) = if elapsed < params.accel_start_ms {
            (step as u32, params.initial_interval_ms)
        } else if elapsed < params.accel_max_ms {
            // Linear interpolation between initial and max speed
            let t = (elapsed - params.accel_start_ms) as f64
                / (params.accel_max_ms - params.accel_start_ms) as f64;
            let interval_f = params.initial_interval_ms as f64
                - t * (params.initial_interval_ms - params.min_interval_ms) as f64;
            let delta_f = step as f64 + t * (step * 4 - step) as f64;
            (delta_f as u32, interval_f as u64)
        } else {
            (step as u32 * 4, params.min_interval_ms)
        };

        send(delta as i32 * direction, horizontal);
        std::thread::sleep(Duration::from_millis(interval));
    }

    // --- Smooth stop ---
    let steps = (params.smooth_stop_ms / 25).max(1);
    let step_delta = step / steps as i32;
    for i in (0..steps).rev() {
        if gen.load(Ordering::SeqCst) != my_gen { return; }
        send((step_delta * (i as i32 + 1)) * direction, horizontal);
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
