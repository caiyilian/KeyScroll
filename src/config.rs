// KeyScroll configuration module - raw types (no windows crate deps)

use std::path::PathBuf;

const DEFAULT_TOML: &str = r##"# KeyScroll configuration
[hotkeys]
scroll_up = "Ctrl+Up"
scroll_down = "Ctrl+Down"
scroll_left = "Ctrl+Shift+Up"
scroll_right = "Ctrl+Shift+Down"

[scroll]
initial_interval_ms = 80
min_interval_ms = 20
acceleration_start_ms = 500
acceleration_max_ms = 3000
step_size = 120

[behavior]
smooth_stop_ms = 100
"##;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub hotkeys: HotkeyConfig,
    pub scroll: ScrollConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct HotkeyConfig {
    pub scroll_up: String,
    pub scroll_down: String,
    pub scroll_left: String,
    pub scroll_right: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScrollConfig {
    pub initial_interval_ms: u64,
    pub min_interval_ms: u64,
    pub acceleration_start_ms: u64,
    pub acceleration_max_ms: u64,
    pub step_size: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BehaviorConfig {
    pub smooth_stop_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_TOML).expect("Default config valid")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HotkeyBinding {
    pub modifiers: u32,
    pub vk: u32,
}

pub fn parse_hotkey(s: &str) -> Result<HotkeyBinding, String> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err("empty".into());
    }
    let mut mods: u32 = 0;
    let key = parts[parts.len() - 1];
    for m in &parts[..parts.len() - 1] {
        match *m {
            "Ctrl" | "Control" => mods |= 2,
            "Shift" => mods |= 4,
            "Alt" => mods |= 1,
            "Win" | "Meta" | "Super" => mods |= 8,
            _ => return Err(format!("unknown mod: {}", m)),
        }
    }
    let vk = match key {
        "Up" => 0x26, "Down" => 0x28, "Left" => 0x25, "Right" => 0x27,
        "Space" => 0x20, "PageUp" => 0x21, "PageDown" => 0x22,
        "Home" => 0x24, "End" => 0x23,
        s if s.len() == 1 => {
            let b = s.as_bytes()[0];
            if b.is_ascii_alphabetic() { b.to_ascii_uppercase() as u32 }
            else { return Err(format!("unknown key: {}", key)); }
        }
        f if f.len() > 1 && f.starts_with('F') => {
            let n: u32 = f[1..].parse().map_err(|_| format!("bad F-key: {}", f))?;
            if n < 1 || n > 24 { return Err("F-key out of range".into()); }
            0x70 + n - 1
        }
        _ => return Err(format!("unknown key: {}", key)),
    };
    Ok(HotkeyBinding { modifiers: mods, vk })
}

pub fn load_config() -> (Config, PathBuf) {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--config") {
        if let Some(p) = args.get(pos + 1) {
            let path = PathBuf::from(p);
            if path.exists() {
                if let Ok(c) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = toml::from_str::<Config>(&c) {
                        return (cfg, path);
                    }
                }
            }
        }
    }

    let local = if let Ok(exe) = std::env::current_exe() {
        let mut p = exe; p.pop(); p.push("config.toml"); p
    } else { PathBuf::from("config.toml") };
    if local.exists() {
        if let Ok(c) = std::fs::read_to_string(&local) {
            if let Ok(cfg) = toml::from_str::<Config>(&c) {
                return (cfg, local);
            }
        }
    }

    let appdata = std::env::var_os("APPDATA")
        .map(|a| { let mut p = PathBuf::from(a); p.push("keyscroll"); p.push("config.toml"); p })
        .unwrap_or_else(|| PathBuf::from("config.toml"));
    if appdata.exists() {
        if let Ok(c) = std::fs::read_to_string(&appdata) {
            if let Ok(cfg) = toml::from_str::<Config>(&c) {
                return (cfg, appdata);
            }
        }
    }

    // Create default
    if let Some(parent) = appdata.parent() { let _ = std::fs::create_dir_all(parent); }
    let _ = std::fs::write(&appdata, DEFAULT_TOML);
    (Config::default(), appdata)
}
