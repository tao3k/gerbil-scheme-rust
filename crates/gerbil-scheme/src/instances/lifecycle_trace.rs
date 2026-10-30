//! Explicit startup-only diagnostics; no request-path clocks or Scheme calls.
use std::{sync::OnceLock, time::Instant};
static TRACE: OnceLock<Option<Instant>> = OnceLock::new();
pub(super) fn event(id: u64, phase: &str) {
    let enabled = TRACE.get_or_init(|| {
        std::env::var_os("GERBIL_INSTANCE_LIFECYCLE_TRACE").map(|_| Instant::now())
    });
    if let Some(start) = enabled {
        eprintln!(
            "GSR-LIFECYCLE id={id} phase={phase} elapsed_us={}",
            start.elapsed().as_micros()
        );
    }
}
