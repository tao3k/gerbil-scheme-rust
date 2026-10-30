//! Opt-in request entry receipts; no Scheme value or scheduling authority escapes.
use super::service_wake::Wake;
use std::sync::{Arc, Mutex};
use std::time::Instant;
/// Timestamp order in the entry snapshot; execution may precede caller wake return.
#[derive(Clone, Copy, Debug)]
pub enum SchemeServiceEntryPhase {
    Route,
    ClassReady,
    CapacityReady,
    Published,
    WakeStart,
    WakeEnd,
    Dequeued,
    OwnerTurn,
    Construct,
}
/// Caller/owner observations, not an isolated OS scheduling measurement.
#[derive(Clone, Debug, Default)]
pub struct SchemeServiceEntryObservation {
    pub marks: [Option<Instant>; 9],
    pub wake: Option<super::SchemeWakeDelivery>,
    pub owner_spans: Vec<super::SchemeServiceOwnerSpan>,
    pub history_lost: bool,
}
/// Explicit per-request observer. Unobserved requests read no entry clocks.
#[derive(Debug, Default)]
pub struct SchemeServiceEntryTrace(Mutex<SchemeServiceEntryObservation>);
impl SchemeServiceEntryTrace {
    /// Snapshot owned Rust metadata; no native entry is performed.
    #[must_use]
    pub fn snapshot(&self) -> SchemeServiceEntryObservation {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(super) fn owner(&self, monitor: &super::service_metrics::Monitor) {
        let mut row = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let (Some(start), Some(end)) = (row.marks[3], row.marks[6]) {
            (row.owner_spans, row.history_lost) = monitor.history.spans(start, end);
        }
    }
    pub(super) fn mark(&self, phase: SchemeServiceEntryPhase) {
        let at = Instant::now();
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .marks[phase as usize] = Some(at);
    }
}
pub(super) fn notify(
    wake: Option<Arc<Wake>>,
    enqueued: bool,
    trace: Option<Arc<SchemeServiceEntryTrace>>,
    short: bool,
) {
    if !enqueued {
        return;
    }
    let Some(trace) = trace else {
        if let Some(wake) = wake {
            wake.notify_for(short);
        }
        return;
    };
    trace.mark(SchemeServiceEntryPhase::WakeStart);
    if let Some(wake) = wake {
        let delivery = wake.notify_for(short);
        trace
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .wake = Some(delivery);
    }
    trace.mark(SchemeServiceEntryPhase::WakeEnd);
}
