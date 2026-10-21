//! Paired request entry marks. Wake return can follow dequeue/execution.
use super::workload::Reply;
use serde_json::{Value, json};
use std::time::Instant;
pub(super) fn receipt(
    reply: &Reply,
    start: Instant,
    anchor: Option<&super::native_wait::Anchor>,
) -> Option<Value> {
    let row = reply.entry.as_ref()?.snapshot();
    Some(
        json!({"owner":reply.owner,"native_clock":anchor.map(|anchor|anchor.marks(&row.marks)),"marks_ns":row.marks.map(|at| at.map(|at|at.saturating_duration_since(start).as_nanos())),
        "wake":row.wake.map(|value|format!("{value:?}")),"history_lost":row.history_lost,
        "owner_spans":row.owner_spans.iter().map(|span|json!({"phase":format!("{:?}",span.phase),
            "overlap_ns":span.overlap_ns,"phase_wall_ns":span.phase_wall_ns,"phase_cpu_ns":span.phase_cpu_ns,"phase_gc_ns":span.phase_gc_ns,"phase_gc_cpu_ns":span.phase_gc_cpu_ns})).collect::<Vec<_>>()}),
    )
}
