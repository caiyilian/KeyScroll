// KeyScroll configuration module
// Loads TOML config from: --config path > local config.toml > %APPDATA%/keyscroll/config.toml > defaults

use std::path::PathBuf;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_SHIFT, MOD_ALT, MOD_WIN,
    VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_SPACE,
    VK_PRIOR, VK_NEXT, VK_HOME, VK_END,
};

const DEFAULT_CONFIG_TOML: &str = r##"# KeyScroll configuration
# Restart KeyScroll after changing this file.

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
        toml::from_str(DEFAULT_CONFIG_TOML).expect("Default config is valid TOML")
    }
}

/// Parsed hotkey binding with modifier flags and virtual key code
#[derive(Debug, Clone, Copy)]
pub struct HotkeyBinding {
    pub modifiers: HOT_KEY_MODIFIERS,
    pub vk: u32,
}

/// Parse a hotkey string like "Ctrl+Up" or "Alt+Shift+F1" into a HotkeyBinding
pub fn parse_hotkey(s: &str) -> Result<HotkeyBinding, String> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err("empty hotkey string".into());
    }

    let mut modifiers = HOT_KEY_MODIFIERS::default();
    let key_name = parts[parts.len() - 1];

    // Parse modifiers from all but the last part
    for mod_name in &parts[..parts.len() - 1] {
        match *mod_name {
            "Ctrl" | "Control" => modifiers |= MOD_CONTROL,
            "Shift" => modifiers |= MOD_SHIFT,
            "Alt" => modifiers |= MOD_ALT,
            "Win" | "Meta" | "Super" => modifiers |= MOD_WIN,
            other => return Err(format!("unknown modifier: {}", other)),
        }
    }

    // Parse key name to virtual key code
    let vk = match key_name {
        "Up" => VK_UP.0 as u32,
        "Down" => VK_DOWN.0 as u32,
        "Left" => VK_LEFT.0 as u32,
        "Right" => VK_RIGHT.0 as u32,
        "Space" => VK_SPACE.0 as u32,
        "PageUp" => VK_PRIOR.0 as u32,
        "PageDown" => VK_NEXT.0 as u32,
        "Home" => VK_HOME.0 as u32,
        "End" => VK_END.0 as u32,
        // Single letters
        s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic() => {
            s.as_bytes()[0].to_ascii_uppercase() as u32
        }
        // Function keys F1-F24
        f if f.len() > 1 && f.starts_with('F') => {
            let num: u32 = f[1..].parse().map_err(|_| format!("unknown key: {}", key_name))?;
            if num < 1 || num > 24 {
                return Err(format!("F-key out of range: F{}", num));
            }
            0x70 + num - 1 // VK_F1 = 0x70
        }
        other => return Err(format!("unknown key: {}", other)),
    };

    Ok(HotkeyBinding { modifiers, vk })
}

/// Load configuration with priority: --config arg > local file > appdata file > defaults
pub fn load_config() -> (Config, PathBuf) {
    // 1. Check --config argument
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--config") {
        if let Some(path_str) = args.get(pos + 1) {
            let path = PathBuf::from(path_str);
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                match toml::from_str::<Config>(&content) {
                    Ok(cfg) => return (cfg, path),
                    Err(e) => eprintln!("Warning: config file '{}' parse error: {}. Using defaults.", path.display(), e),
                }
            } else {
                eprintln!("Warning: config file '{}' not found. Using defaults.", path.display());
            }
        }
    }

    // 2. Check local config.toml (alongside the executable)
    let local_path = if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path;
        p.pop();
        p.push("config.toml");
        p
    } else {
        PathBuf::from("config.toml")
    };

    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path).unwrap_or_default();
        match toml::from_str::<Config>(&content) {
            Ok(cfg) => {
                eprintln!("KeyScroll: loaded config from {}", local_path.display());
                return (cfg, local_path);
            }
            Err(e) => eprintln!("Warning: local config parse error: {}. Trying appdata config.", e),
        }
    }

    // 3. Check %APPDATA%/keyscroll/config.toml
    let appdata_path = if let Some(appdata) = std::env::var_os("APPDATA") {
        let mut p = PathBuf::from(appdata);
        p.push("keyscroll");
        p.push("config.toml");
        p
    } else {
        PathBuf::from("config.toml")
    };

    if appdata_path.exists() {
        let content = std::fs::read_to_string(&appdata_path).unwrap_or_default();
        match toml::from_str::<Config>(&content) {
            Ok(cfg) => {
                eprintln!("KeyScroll: loaded config from {}", appdata_path.display());
                return (cfg, appdata_path);
            }
            Err(e) => eprintln!("Warning: appdata config parse error: {}. Using defaults.", e),
        }
    }

    // 4. Create default config at appdata path
    if let Some(parent) = appdata_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&appdata_path, DEFAULT_CONFIG_TOML) {
        eprintln!("Warning: could not write default config to {}: {}", appdata_path.display(), e);
    } else {
        eprintln!("KeyScroll: created default config at {}", appdata_path.display());
    }

    (Config::default(), appdata_path)
}
