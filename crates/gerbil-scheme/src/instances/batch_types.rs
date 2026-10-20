//! Optional wall-clock markers at actual native batch boundaries.
use std::time::Instant;
#[derive(Clone, Copy, Debug)]
pub(super) enum BatchPhase {
    ItemsConstructed,
    Constructed,
    Executed,
    Projected,
}
/// Owner-local phase markers. Durations include OS preemption within each span.
/// `dispatched` precedes admission, so dispatch-to-enter includes Rust admission
/// overhead as well as queue residence. Ticket completion additionally includes
/// root release and balance checking after projection.
#[derive(Clone, Copy, Debug)]
pub struct BatchTimeline {
    pub dispatched: Instant,
    pub entered: Instant,
    /// All per-item numeric vectors are rooted; collection/envelope not built yet.
    pub items_constructed: Instant,
    pub constructed: Instant,
    pub executed: Instant,
    pub projected: Instant,
    /// Native GC counts at entry, constructed, executed and projected markers.
    pub collections: [u64; 4],
    /// Optional diagnostics at entry, item construction, envelope construction,
    /// Scheme return and projection; see `NativeSession::diagnostics` for units.
    pub diagnostics: [Option<[f64; 6]>; 5],
}
/// An owned native result with opt-in phase markers; no Scheme reference escapes.
#[derive(Debug)]
pub struct ProfiledBatch<T> {
    pub values: Vec<Vec<T>>,
    pub timeline: BatchTimeline,
}
