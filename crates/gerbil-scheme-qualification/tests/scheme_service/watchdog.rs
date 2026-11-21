//! Failure evidence bypasses libtest capture without changing progress deadlines.
use std::time::Duration;
pub(super) const GATE: Duration = Duration::from_secs(5);
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
