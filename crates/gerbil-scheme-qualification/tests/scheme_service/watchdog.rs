//! Failure evidence bypasses libtest capture without changing progress deadlines.
use super::GATE;
use std::{io::Write, sync::mpsc, time::Instant};
pub(super) fn observe(events: &mpsc::Receiver<&'static str>) {
    let mut last = "start";
    let mut phase = "start";
    let mut deadline = Instant::now() + GATE;
    loop {
        match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(event) => {
                if let Some(next) = event.strip_prefix("PHASE ") {
                    phase = next;
                } else {
                    println!("SERVICE-PROGRESS {event}");
                    last = event;
                    deadline = Instant::now() + GATE;
                }
                // Phase chatter is diagnostic context, never a progress heartbeat.
                if Instant::now() >= deadline {
                    stall(last, phase);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => stall(last, phase),
        }
    }
}
fn stall(last: &str, phase: &str) -> ! {
    let _ = writeln!(
        std::io::stderr().lock(),
        "SERVICE-STALL last={last} phase={phase} cleanup=UNVERIFIED"
    );
    std::process::exit(70);
}
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
        observe(&receive);
        panic!("injected stalled operation must not pass");
    }
    let start = Instant::now();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "watchdog::failure_retains_phase_without_renewing_progress",
        ])
        .env(CHILD, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() >= GATE * 2 {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("watchdog failed to enforce the unchanged progress deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(70));
    assert!(start.elapsed() >= GATE && start.elapsed() < GATE * 2);
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("SERVICE-STALL last=injected checkpoint phase=injected blocked native operation cleanup=UNVERIFIED"), "{error}");
}
