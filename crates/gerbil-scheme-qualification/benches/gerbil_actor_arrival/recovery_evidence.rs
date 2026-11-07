//! Test-only cold analysis retained from the rejected peak observer.
use super::wait_evidence::{ClockCut, gap_cpu_bounds};

fn source_point(value: &serde_json::Value) -> Option<(u64, u64, u64, u64, u64)> {
    if value.get("valid")?.as_u64()? != 1 {
        return None;
    }
    let read = |name: &str| value.get(name)?.as_u64();
    let (due, published, consumed) = (read("due")?, read("published")?, read("consumed")?);
    if consumed > published || published > due {
        return None;
    }
    Some((read("wall")?, due, published, consumed, read("ordinal")?))
}

fn recovery_bounds(
    start: &serde_json::Value,
    end: &serde_json::Value,
) -> Option<serde_json::Value> {
    let cut = |value: &serde_json::Value, prefix: &str| {
        if value.get(format!("{prefix}-valid"))?.as_u64()? != 1 {
            return None;
        }
        let read = |name: &str| value.get(format!("{prefix}-{name}"))?.as_u64();
        Some((
            ClockCut {
                begin: read("begin")?,
                end: read("end")?,
                cpu: read("cpu")?,
            },
            read("gc_count")?,
            read("gc_wall")?,
            read("gc_cpu")?,
        ))
    };
    let before = cut(start, "before")?;
    let after = cut(end, "after")?;
    let bounds = gap_cpu_bounds(
        true,
        before.0,
        after.0,
        source_point(start)?.0,
        source_point(end)?.0,
    )?;
    Some(
        serde_json::json!({"wall_ns":bounds.elapsed,"owner_cpu_upper_ns":bounds.cpu_upper,
        "non_cpu_lower_ns":bounds.non_cpu_lower,
        "enclosing_cut_wall_ns":after.0.end.checked_sub(before.0.begin)?,
        "enclosing_gc_count":after.1.checked_sub(before.1)?,
        "enclosing_gc_wall_ns":after.2.checked_sub(before.2)?,
        "enclosing_gc_cpu_ns":after.3.checked_sub(before.3)?,
        "select_coverage":"not-recorded-for-this-recovery-episode",
        "kernel_blocked_ns":null,"os_scheduling_wait_ns":null}),
    )
}

fn recovery_flow(start: &serde_json::Value, end: &serde_json::Value) -> Option<serde_json::Value> {
    let a = source_point(start)?;
    let b = source_point(end)?;
    let wall = b.0.checked_sub(a.0)?;
    if wall == 0 || b.4 < a.4 {
        return None;
    }
    let arrivals = b.1.checked_sub(a.1)?;
    let published = b.2.checked_sub(a.2)?;
    let consumed = b.3.checked_sub(a.3)?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "observed rates accompany exact integer counts and times"
    )]
    let rate = |count: u64| count as f64 * 1e9 / wall as f64;
    Some(
        serde_json::json!({"elapsed_ns":wall,"arrivals":arrivals,"published":published,"consumed":consumed,
        "source_debt_begin":a.1-a.2,"source_debt_end":b.1-b.2,
        "outstanding_begin":a.1-a.3,"outstanding_end":b.1-b.3,
        "consumption_surplus_count":consumed.saturating_sub(arrivals),
        "consumption_deficit_count":arrivals.saturating_sub(consumed),
        "publication_surplus_count":published.saturating_sub(arrivals),
        "observed_arrival_per_second":rate(arrivals),
        "observed_publication_per_second":rate(published),
        "observed_consumption_per_second":rate(consumed),
        "cpu_gc_bounds":recovery_bounds(start,end)}),
    )
}

fn peak_bracket(peak: &serde_json::Value) -> Option<serde_json::Value> {
    let field = |name: &str| peak.get(name)?.as_u64();
    if field("before-valid")? != 1
        || field("after-valid")? != 1
        || field("before-begin")? > field("before-end")?
        || field("after-begin")? > field("after-end")?
        || field("before-end")? > field("wall")?
        || field("wall")? > field("after-begin")?
    {
        return None;
    }
    Some(
        serde_json::json!({"scope":"encloses-the-actual-source-peak-not-an-exclusive-stage",
        "enclosing_wall_ns":field("after-end")?.checked_sub(field("before-begin")?)?,
        "owner_cpu_ns":field("after-cpu")?.checked_sub(field("before-cpu")?)?,
        "gc_count":field("after-gc_count")?.checked_sub(field("before-gc_count")?)?,
        "gc_wall_ns":field("after-gc_wall")?.checked_sub(field("before-gc_wall")?)?,
        "gc_cpu_ns":field("after-gc_cpu")?.checked_sub(field("before-gc_cpu")?)?}),
    )
}

/// The selected peak, its preceding below-guard source check, and first
/// recovered source check belong to one episode. The largest gap is irrelevant.
pub(super) fn recovery_report(
    s: &serde_json::Value,
    offered_end: Option<u64>,
) -> Option<serde_json::Value> {
    let read = |name: &str| s.get(name)?.as_u64();
    if read("generation")? == 0
        || read("complete")? != 1
        || read("invalid")? != 0
        || read("aborted")? != 0
        || read("error")? != 0
    {
        return None;
    }
    let recovery = s.get("recovery")?;
    if recovery.get("invalid")?.as_u64()? != 0 || recovery.get("episode")?.as_u64()? == 0 {
        return None;
    }
    let guard = recovery.get("guard")?.as_u64()?;
    let start = recovery.get("start")?;
    let peak = recovery.get("peak")?;
    let end = recovery.get("end")?;
    let a = source_point(start)?;
    let b = source_point(peak)?;
    if guard == 0
        || a.1 - a.3 > guard
        || b.1 - b.3 <= guard
        || b != (
            read("peak-wall")?,
            read("peak-due")?,
            read("peak-published")?,
            read("peak-consumed")?,
            read("peak-progress-ordinal")?,
        )
    {
        return None;
    }
    let growth = recovery_flow(start, peak)?;
    let recovered = source_point(end).filter(|c| c.1 - c.3 <= guard);
    let schedule_relation = match (recovered, offered_end) {
        (Some(point), Some(limit)) if point.0 <= limit => "recovered-before-arrival-stop",
        (Some(_), Some(_)) => "drain-crosses-arrival-stop",
        _ => "not-established",
    };
    Some(
        serde_json::json!({"kind":"actual-peak-source-recovery","generation":read("generation")?,
        "episode":recovery.get("episode")?,"guard":guard,
        "peak_source_unpublished":b.1-b.2,"peak_published_unconsumed":b.2-b.3,
        "peak_cpu_gc_bracket":peak_bracket(peak),
        "growth":growth,"recovery":recovered.and_then(|_|recovery_flow(peak,end)),
        "arrival_schedule_relation":schedule_relation,
        "phase_bounds_overlap_possible":true,"maximum_capacity_measured":false}),
    )
}
