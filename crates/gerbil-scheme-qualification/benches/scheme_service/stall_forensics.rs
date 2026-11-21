//! Opt-in terminal diagnostics, only after the unchanged watchdog declares failure.
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};
static ENABLED: OnceLock<bool> = OnceLock::new();
static PHASE: AtomicUsize = AtomicUsize::new(0);
static CONTEXT: AtomicUsize = AtomicUsize::new(usize::MAX);
pub(super) fn phase(phase: usize) {
    if *ENABLED.get_or_init(|| std::env::var_os("GERBIL_SERVICE_STALL_SAMPLE").is_some()) {
        PHASE.store(phase, Ordering::Release);
    }
}
pub(super) fn context(block: usize, reverse: bool, mixed: bool, arm: usize) {
    if *ENABLED.get_or_init(|| std::env::var_os("GERBIL_SERVICE_STALL_SAMPLE").is_some()) {
        CONTEXT.store(
            block
                .saturating_mul(8)
                .saturating_add(usize::from(reverse) * 4 + usize::from(mixed) * 2 + arm),
            Ordering::Release,
        );
    }
}
pub(super) fn capture() {
    let Some(path) = std::env::var_os("GERBIL_SERVICE_STALL_SAMPLE") else {
        return;
    };
    let context = CONTEXT.load(Ordering::Acquire);
    eprintln!(
        "SERVICE-BENCH-FAILURE-CONTEXT block={} reverse={} mixed={} arm={} diagnostic-only=true",
        context / 8,
        context & 4 != 0,
        context & 2 != 0,
        context & 1
    );
    let phases = [
        "unknown",
        "idle-before-config",
        "router",
        "configure",
        "idle-after-config",
        "short-wake",
        "idle-after-short-wake",
        "counter-snapshots",
        "clients",
        "idle-after-clients",
        "summary",
        "policy-readback",
        "validated",
        "projection-gate-entry",
        "registration-pending",
        "projection-released",
        "replacement-registration",
        "replacement-result",
        "producer-retirement",
        "idle-policy-reset",
        "post-reset-execution",
        "old-client-consumption",
        "capacity-occupy",
        "short-publish-zero-capacity",
        "short-wait-after-capacity-release",
        "bulk-client-consumption",
        "source-entry-gates",
        "source-publisher-paused",
        "source-native-progress",
        "source-retirement-while-open",
        "source-publisher-resumed",
        "source-client-consumption",
    ];
    eprintln!(
        "SERVICE-BENCH-FAILURE-PHASE {}",
        phases
            .get(PHASE.load(Ordering::Acquire))
            .unwrap_or(&"invalid")
    );
    sample_self(&path);
}
#[cfg(target_os = "macos")]
fn sample_self(path: &std::ffi::OsStr) {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let diagnostics = std::path::PathBuf::from(path).with_extension("sample-stderr.txt");
    let stderr = std::fs::File::create(diagnostics).map_or_else(|_| Stdio::null(), Stdio::from);
    let child = Command::new("/usr/bin/sample")
        .arg(std::process::id().to_string())
        .args(["1", "1", "-mayDie", "-file"])
        .arg(path)
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn();
    let Ok(mut child) = child else {
        eprintln!("SERVICE-BENCH-FORENSIC sample=spawn-failed");
        return;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("SERVICE-BENCH-FORENSIC sample-exit={status}");
                return;
            }
            Err(_) => break,
            // Failure was already declared; this bounds diagnostics, not execution.
            Ok(None) if started.elapsed() >= Duration::from_secs(15) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    eprintln!("SERVICE-BENCH-FORENSIC sample=unavailable-or-timeout");
}
#[cfg(not(target_os = "macos"))]
fn sample_self(_: &std::ffi::OsStr) {
    eprintln!("SERVICE-BENCH-FORENSIC sample=unsupported-host");
}
