//! Bounded diagnostic history for overlap with a request's native inbox wait.
use super::SchemeServicePhase;
use std::{collections::VecDeque, sync::Mutex, time::Instant};
/// Full phase CPU/GC counters enclose the overlap; they are not clipped CPU time.
#[derive(Clone, Copy, Debug)]
pub struct SchemeServiceOwnerSpan {
    pub phase: SchemeServicePhase,
    pub overlap_ns: u64,
    pub phase_wall_ns: u64,
    pub phase_cpu_ns: Option<u64>,
    pub phase_gc_ns: Option<u64>,
    /// SDK process CPU over GC intervals; includes other host threads.
    pub phase_gc_cpu_ns: Option<u64>,
}
#[derive(Debug)]
struct Record {
    phase: SchemeServicePhase,
    start: Instant,
    end: Instant,
    delta: Option<[u64; 3]>,
}
#[derive(Debug, Default)]
struct Records {
    rows: VecDeque<Record>,
    lost_before: Option<Instant>,
}
#[derive(Debug, Default)]
pub(super) struct History(Mutex<Records>);
fn ns(start: Instant, end: Instant) -> u64 {
    u64::try_from(end.duration_since(start).as_nanos()).unwrap_or(u64::MAX)
}
impl History {
    pub(super) fn record(
        &self,
        phase: SchemeServicePhase,
        start: Instant,
        end: Instant,
        delta: Option<[u64; 3]>,
    ) {
        let mut rows = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if rows.rows.len() == 256 {
            rows.lost_before = rows.rows.pop_front().map(|row| row.end);
        }
        rows.rows.push_back(Record {
            phase,
            start,
            end,
            delta,
        });
    }
    pub(super) fn spans(
        &self,
        start: Instant,
        end: Instant,
    ) -> (Vec<SchemeServiceOwnerSpan>, bool) {
        let rows = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let spans = rows
            .rows
            .iter()
            .filter(|row| row.end > start && row.start < end)
            .map(|row| SchemeServiceOwnerSpan {
                phase: row.phase,
                overlap_ns: ns(row.start.max(start), row.end.min(end)),
                phase_wall_ns: ns(row.start, row.end),
                phase_cpu_ns: row.delta.map(|delta| delta[0]),
                phase_gc_ns: row.delta.map(|delta| delta[1]),
                phase_gc_cpu_ns: row.delta.map(|delta| delta[2]),
            })
            .collect();
        (spans, rows.lost_before.is_some_and(|at| at > start))
    }
}
