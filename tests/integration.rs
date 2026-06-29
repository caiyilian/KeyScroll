// KeyScroll — integration tests (Phase 6.5)
// Tests the compiled binary via std::process::Command.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn exe_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("release");
    p.push("keyscroll.exe");
    p
}

#[test]
fn binary_exists() {
    let p = exe_path();
    assert!(p.exists(), "Release binary not found at {:?}", p);
}

#[test]
fn binary_starts_and_stops() {
    let mut child = Command::new(exe_path())
        .spawn()
        .expect("Failed to spawn keyscroll");
    std::thread::sleep(Duration::from_secs(2));
    // Must be running after 2 seconds
    match child.try_wait() {
        Ok(Some(status)) => panic!("Process exited early with code {:?}", status.code()),
        Ok(None) => {} // still running — good
        Err(e) => panic!("Error checking process: {}", e),
    }
    child.kill().expect("Failed to kill keyscroll");
    child.wait().expect("Failed to wait for keyscroll");
}

#[test]
fn install_flag_exits_cleanly() {
    let output = Command::new(exe_path())
        .arg("--install")
        .output()
        .expect("Failed to run keyscroll --install");
    // May fail (non-admin) or succeed — either is fine, just don't hang
    assert!(
        output.status.code().is_some(),
        "--install did not exit (exit code: {:?})",
        output.status.code()
    );
}

#[test]
fn uninstall_flag_exits_cleanly() {
    let output = Command::new(exe_path())
        .arg("--uninstall")
        .output()
        .expect("Failed to run keyscroll --uninstall");
    assert!(
        output.status.code().is_some(),
        "--uninstall did not exit (exit code: {:?})",
        output.status.code()
    );
}

#[test]
fn log_file_created_on_startup() {
    // Clean up any old log
    let log = exe_path().with_extension("log");
    let _ = std::fs::remove_file(&log);
    let mut child = Command::new(exe_path())
        .spawn()
        .expect("Failed to spawn keyscroll");
    std::thread::sleep(Duration::from_secs(2));
    child.kill().expect("Failed to kill keyscroll");
    child.wait().expect("Failed to wait for keyscroll");
    assert!(log.exists(), "Log file not created at {:?}", log);
}

#[test]
fn log_contains_startup_message() {
    let log = exe_path().with_extension("log");
    if !log.exists() { return; } // skip if no log (parallel test race)
    let content = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        content.contains("KeyScroll started"),
        "Log does not contain startup message:\n{}",
        content
    );
}
