//! Real loopback/process checks, opt-in on hosts that permit listener creation.
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Service(Child);
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(root: &std::path::Path, idle: &str) -> Service {
    Service(
        Command::new(env!("CARGO_BIN_EXE_brickxinator-app"))
            .args(["serve", "--data-dir"])
            .arg(root)
            .args(["--port", "0", "--idle-seconds", idle])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    )
}
fn ready(root: &std::path::Path) {
    let start = Instant::now();
    while !root.join("runtime.json").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "service did not become ready"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
#[test]
#[ignore = "requires permission to bind a loopback port"]
fn second_launcher_reuses_service_and_idle_shutdown_cleans_runtime() {
    let root = tempfile::tempdir().unwrap();
    let mut service = start(root.path(), "2");
    ready(root.path());
    let result = Command::new(env!("CARGO_BIN_EXE_brickxinator-app"))
        .args(["serve", "--data-dir"])
        .arg(root.path())
        .args(["--port", "0"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("already running at http://127.0.0.1:")
    );
    let deadline = Instant::now();
    loop {
        if let Some(status) = service.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(deadline.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(!root.path().join("runtime.json").exists());
}
#[test]
#[ignore = "requires permission to bind a loopback port"]
fn crashed_service_releases_os_lock_and_next_launcher_replaces_stale_runtime() {
    let root = tempfile::tempdir().unwrap();
    let mut first = start(root.path(), "60");
    ready(root.path());
    let old = std::fs::read(root.path().join("runtime.json")).unwrap();
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let mut second = start(root.path(), "1");
    let deadline = Instant::now();
    loop {
        if std::fs::read(root.path().join("runtime.json")).is_ok_and(|bytes| bytes != old) {
            break;
        }
        assert!(deadline.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(second.0.wait().unwrap().success());
}
