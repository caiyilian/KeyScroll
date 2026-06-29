// KeyScroll — lightweight file logger (Phase 5.3)

use std::fs::{File, OpenOptions, rename};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

const MAX_LOG_SIZE: u64 = 1_048_576; // 1 MB
const TRIM_TARGET: u64 = 262_144;    // keep 256 KB after rotation

pub struct Logger {
    file: Mutex<File>,
    path: PathBuf,
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap_or_else(|_| {
                // Fallback: create a fresh file if append fails
                File::create(&path).expect("Cannot create log file")
            });
        Logger { file: Mutex::new(file), path }
    }

    pub fn event(&self, msg: &str) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let (sec, ns) = (now, 0);
        // Build a simple timestamp: [HH:MM:SS]
        let h = (sec / 3600) % 24;
        let m = (sec / 60) % 60;
        let s = sec % 60;
        let ts = format!("[{:02}:{:02}:{:02}]", h, m, s);

        let line = format!("{} {}\r\n", ts, msg);
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
            // Check size and rotate if needed
            if let Ok(meta) = f.metadata() {
                if meta.len() > MAX_LOG_SIZE {
                    // Rename current log -> .old, then reopen fresh
                    let old = self.path.with_extension("log.old");
                    let _ = rename(&self.path, &old);
                    // Truncate .old to TRIM_TARGET
                    if let Ok(of) = OpenOptions::new().write(true).open(&old) {
                        if let Ok(meta) = of.metadata() {
                            if meta.len() > TRIM_TARGET {
                                let _ = of.set_len(TRIM_TARGET);
                            }
                        }
                    }
                    // Reopen fresh log
                    if let Ok(nf) = File::create(&self.path) {
                        *f = nf;
                    }
                }
            }
        }
    }
}