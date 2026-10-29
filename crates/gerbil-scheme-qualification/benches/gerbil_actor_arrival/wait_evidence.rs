//! Cold arithmetic for one closed owner report. All values use nanoseconds.
//! A wrapper call interval does not identify kernel blocking or OS scheduling.

#[derive(Clone, Copy)]
pub(super) struct ClockCut {
    pub begin: u64,
    pub end: u64,
    pub cpu: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CpuBounds {
    pub elapsed: u64,
    pub cpu_upper: u64,
    pub non_cpu_lower: u64,
}

/// The caller must take every field from one closed owner report. This does
/// not combine a remote sample, an earlier arm or an unrelated recovery window.
pub(super) fn gap_cpu_bounds(
    same_interval: bool,
    before: ClockCut,
    after: ClockCut,
    begin: u64,
    end: u64,
) -> Option<CpuBounds> {
    if !same_interval
        || before.begin > before.end
        || before.end > begin
        || begin >= end
        || end > after.begin
        || after.begin > after.end
    {
        return None;
    }
    let cpu_upper = after.cpu.checked_sub(before.cpu)?;
    let elapsed = call_overlap(true, (begin, end), (before.end, after.begin))?;
    Some(CpuBounds {
        elapsed,
        cpu_upper,
        non_cpu_lower: elapsed.saturating_sub(cpu_upper),
    })
}

/// Intersection with a completed wrapper span, not a blocked-time estimate.
/// No record (or an uncovered backend) cannot establish absence of blocking.
pub(super) fn call_overlap(same_scope: bool, gap: (u64, u64), call: (u64, u64)) -> Option<u64> {
    if !same_scope || gap.0 >= gap.1 || call.0 > call.1 {
        return None;
    }
    Some(gap.1.min(call.1).saturating_sub(gap.0.max(call.0)))
}

// Interpret one closed report only, after returning from the native arm.
pub(super) fn report(stages: &serde_json::Value) -> Option<serde_json::Value> {
    fn cut(value: &serde_json::Value) -> Option<ClockCut> {
        Some(ClockCut {
            begin: value.get("begin_wall_ns")?.as_u64()?,
            end: value.get("end_wall_ns")?.as_u64()?,
            cpu: value.get("thread_cpu_ns")?.as_u64()?,
        })
    }
    if stages.get("generation").is_some() {
        return scoped_report(stages);
    }
    let owner = stages.get("owner")?;
    if owner.get("query_errors")?.as_u64()? != 0 {
        return None;
    }
    let bounds = gap_cpu_bounds(
        owner.get("same_interval")?.as_bool()?,
        cut(owner.get("self_before")?)?,
        cut(owner.get("self_after")?)?,
        owner.get("completion_gap_begin_wall_ns")?.as_u64()?,
        owner.get("completion_gap_end_wall_ns")?.as_u64()?,
    )?;
    Some(serde_json::json!({
        "kind":"verified-progress-gap",
        "gap_wall_ns":bounds.elapsed,
        "owner_cpu_upper_ns":bounds.cpu_upper,
        "wall_without_owner_cpu_lower_ns":bounds.non_cpu_lower,
        "kernel_blocked_ns":null,
        "os_scheduling_wait_ns":null,
        "select_coverage":"not-established-for-this-interval",
        "exclusive_stage_cpu_available":false
    }))
}

// All fields were copied from one explicitly closed generation. Enclosing CPU
// and GC cuts include their sampling skew; neither is an exclusive phase cost.
fn scoped_report(s: &serde_json::Value) -> Option<serde_json::Value> {
    let field = |name: &str| s.get(name)?.as_u64();
    if field("generation")? == 0
        || field("complete")? != 1
        || field("invalid")? != 0
        || field("aborted")? != 0
        || field("error")? != 0
        || field("clock-valid")? != 1
    {
        return None;
    }
    let cut = |prefix: &str| {
        Some(ClockCut {
            begin: field(&format!("{prefix}-begin"))?,
            end: field(&format!("{prefix}-end"))?,
            cpu: field(&format!("{prefix}-cpu"))?,
        })
    };
    let begin = field("gap-begin")?;
    let end = field("gap-end")?;
    let bounds = gap_cpu_bounds(true, cut("before")?, cut("after")?, begin, end)?;
    let call_wall = field("call-wall")?;
    let call_cpu = field("call-cpu")?;
    if call_wall > bounds.elapsed || call_cpu > bounds.cpu_upper {
        return None;
    }
    let peak = field("peak-wall")?;
    let ordinal = field("best-progress-ordinal")?;
    let peak_ordinal = field("peak-progress-ordinal")?;
    let relation = if begin <= peak && peak <= end && peak_ordinal.checked_add(1) == Some(ordinal) {
        "inside-progress-gap"
    } else if peak >= end && peak_ordinal == ordinal {
        "source-check-following-progress"
    } else {
        "not-associated-with-largest-gap"
    };
    Some(serde_json::json!({
        "kind":"explicit-generation-progress-gap", "generation":field("generation")?,
        "gap_wall_ns":bounds.elapsed,"owner_cpu_upper_ns":bounds.cpu_upper,
        "wall_without_owner_cpu_lower_ns":bounds.non_cpu_lower,
        "select_call_wall_ns":call_wall,"select_call_cpu_ns":call_cpu,
        "non_cpu_outside_select_lower_ns":bounds.non_cpu_lower.saturating_sub(call_wall),
        "enclosing_gc_count":field("after-gc-count")?.checked_sub(field("before-gc-count")?)?,
        "enclosing_gc_wall_ns":field("after-gc-wall")?.checked_sub(field("before-gc-wall")?)?,
        "enclosing_gc_cpu_ns":field("after-gc-cpu")?.checked_sub(field("before-gc-cpu")?)?,
        "peak_relation":relation,
        "peak_source_unpublished":field("peak-due")?.checked_sub(field("peak-published")?)?,
        "peak_published_unconsumed":field("peak-published")?.checked_sub(field("peak-consumed")?)?,
        "kernel_blocked_ns":null,"os_scheduling_wait_ns":null,
        "exclusive_stage_cpu_available":false
    }))
}
