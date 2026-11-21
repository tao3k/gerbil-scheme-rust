//! Dedicated process: failure-control pipes never overlap native FD acceptance.
#[path = "scheme_service/watchdog.rs"]
mod watchdog;
use std::{sync::mpsc, time::Instant};
use watchdog::GATE;
#[test]
fn failure_retains_phase_without_renewing_progress() {
    const CHILD: &str = "GERBIL_SERVICE_WATCHDOG_FAILURE_CONTROL";
    if std::env::var_os(CHILD).is_some() {
        let (send, receive) = mpsc::channel();
        send.send("injected checkpoint").unwrap();
        std::thread::spawn(move || {
            loop {
                if send
                    .send("PHASE injected blocked native operation")
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        });
        watchdog::observe(&receive);
        panic!("injected stalled operation must not pass");
    }
    #[cfg(target_os = "macos")]
    let descriptors = std::fs::read_dir("/dev/fd").unwrap().count();
    let start = Instant::now();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "failure_retains_phase_without_renewing_progress"])
        .env(CHILD, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        std::fs::read_dir("/dev/fd").unwrap().count(),
        descriptors + 2,
        "capture pipes belong to this test process, not the Scheme VM"
    );
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() >= GATE * 2 {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("watchdog failed to enforce the unchanged progress deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        std::fs::read_dir("/dev/fd").unwrap().count(),
        descriptors,
        "watchdog capture resources must close after collection"
    );
    assert_eq!(output.status.code(), Some(70));
    assert!(start.elapsed() >= GATE && start.elapsed() < GATE * 2);
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("SERVICE-STALL last=injected checkpoint phase=injected blocked native operation cleanup=UNVERIFIED"), "{error}");
}
