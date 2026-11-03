// Report arithmetic only; this test neither starts a VM nor runs a load.
#[path = "../benches/gerbil_actor_arrival/recovery_evidence.rs"]
mod recovery_evidence;
#[path = "../benches/gerbil_actor_arrival/wait_evidence.rs"]
mod wait_evidence;

use wait_evidence::{ClockCut, CpuBounds, call_overlap, gap_cpu_bounds};

fn before() -> ClockCut {
    ClockCut {
        begin: 1,
        end: 2,
        cpu: 10,
    }
}
fn after() -> ClockCut {
    ClockCut {
        begin: 104,
        end: 105,
        cpu: 12,
    }
}

#[test]
fn matched_enclosing_cpu_is_an_upper_bound() {
    assert_eq!(
        gap_cpu_bounds(true, before(), after(), 3, 103),
        Some(CpuBounds {
            elapsed: 100,
            cpu_upper: 2,
            non_cpu_lower: 98
        })
    );
}

#[test]
fn missing_scope_or_partial_brackets_are_unknown() {
    assert_eq!(gap_cpu_bounds(false, before(), after(), 3, 103), None);
    assert_eq!(gap_cpu_bounds(true, before(), after(), 1, 103), None);
    assert_eq!(gap_cpu_bounds(true, before(), after(), 3, 105), None);
    assert_eq!(call_overlap(false, (3, 103), (20, 40)), None);
}

#[test]
fn reversed_clocks_or_cpu_are_unknown() {
    assert_eq!(
        gap_cpu_bounds(true, before(), ClockCut { cpu: 9, ..after() }, 3, 103),
        None
    );
    assert_eq!(
        gap_cpu_bounds(
            true,
            ClockCut {
                begin: 3,
                ..before()
            },
            after(),
            3,
            103
        ),
        None
    );
    assert_eq!(gap_cpu_bounds(true, before(), after(), 103, 3), None);
    assert_eq!(call_overlap(true, (3, 103), (40, 20)), None);
}

#[test]
fn enclosing_cpu_does_not_become_an_exact_gap_cost() {
    let a = ClockCut {
        cpu: 210,
        ..after()
    };
    assert_eq!(
        gap_cpu_bounds(true, before(), a, 3, 103),
        Some(CpuBounds {
            elapsed: 100,
            cpu_upper: 200,
            non_cpu_lower: 0
        })
    );
}

#[test]
fn call_span_is_clipped_before_any_interpretation() {
    assert_eq!(call_overlap(true, (3, 103), (1, 200)), Some(100));
    assert_eq!(call_overlap(true, (3, 103), (20, 40)), Some(20));
    assert_eq!(call_overlap(true, (3, 103), (104, 105)), Some(0));
    // The same overlap admits 20 units of OS wait OR 20 units of kernel wait.
    // Nothing in this function manufactures a kernel-blocked-time field.
}

#[test]
fn unsigned_bounds_do_not_wrap() {
    let b = ClockCut {
        begin: 0,
        end: 0,
        cpu: 0,
    };
    let a = ClockCut {
        begin: u64::MAX,
        end: u64::MAX,
        cpu: u64::MAX,
    };
    assert_eq!(
        gap_cpu_bounds(true, b, a, 0, u64::MAX),
        Some(CpuBounds {
            elapsed: u64::MAX,
            cpu_upper: u64::MAX,
            non_cpu_lower: 0
        })
    );
    assert_eq!(
        call_overlap(true, (0, u64::MAX), (0, u64::MAX)),
        Some(u64::MAX)
    );
}

fn closed_report() -> serde_json::Value {
    serde_json::json!({"owner":{
        "same_interval":true,"query_errors":0,
        "completion_gap_begin_wall_ns":3,"completion_gap_end_wall_ns":103,
        "self_before":{"begin_wall_ns":1,"end_wall_ns":2,"thread_cpu_ns":10},
        "self_after":{"begin_wall_ns":104,"end_wall_ns":105,"thread_cpu_ns":12},
        "select_samples":100,"select_waiting_samples":100
    }})
}

#[test]
fn report_projection_keeps_blocking_and_scheduling_unknown() {
    let result = wait_evidence::report(&closed_report()).unwrap();
    assert_eq!(result["owner_cpu_upper_ns"], 2);
    assert_eq!(result["wall_without_owner_cpu_lower_ns"], 98);
    assert!(result["kernel_blocked_ns"].is_null());
    assert!(result["os_scheduling_wait_ns"].is_null());
    assert_eq!(
        result["select_coverage"],
        "not-established-for-this-interval"
    );
}

#[test]
fn incomplete_or_failed_report_is_unknown_instead_of_zero() {
    assert!(wait_evidence::report(&serde_json::json!({})).is_none());
    let mut value = closed_report();
    value["owner"]["query_errors"] = 1.into();
    assert!(wait_evidence::report(&value).is_none());
    value = closed_report();
    value["owner"]["same_interval"] = false.into();
    assert!(wait_evidence::report(&value).is_none());
    value = closed_report();
    value["owner"]["self_after"]["thread_cpu_ns"] = (-1).into();
    assert!(wait_evidence::report(&value).is_none());
}

fn scoped_report() -> serde_json::Value {
    serde_json::json!({"generation":2,"complete":1,"invalid":0,"aborted":0,"error":0,"clock-valid":1,
        "gap-begin":3,"gap-end":103,"before-begin":1,"before-end":2,"before-cpu":10,
        "after-begin":104,"after-end":105,"after-cpu":12,"call-wall":20,"call-cpu":1,
        "before-gc-count":4,"after-gc-count":4,"before-gc-wall":20,"after-gc-wall":20,
        "before-gc-cpu":10,"after-gc-cpu":10,"best-progress-ordinal":6,
        "peak-wall":106,"peak-progress-ordinal":6,"peak-due":351,"peak-published":128,"peak-consumed":0})
}

#[test]
fn explicit_generation_separates_source_backlog_and_wait_coverage() {
    let result = wait_evidence::report(&scoped_report()).unwrap();
    assert_eq!(result["peak_source_unpublished"], 223);
    assert_eq!(result["peak_published_unconsumed"], 128);
    assert_eq!(result["non_cpu_outside_select_lower_ns"], 78);
    assert_eq!(result["peak_relation"], "source-check-following-progress");
    assert_eq!(result["enclosing_gc_count"], 0);
    assert!(result["os_scheduling_wait_ns"].is_null());
}

#[test]
fn aborted_or_unbracketed_generation_cannot_become_zero_wait() {
    for name in ["invalid", "aborted", "error"] {
        let mut value = scoped_report();
        value[name] = 1.into();
        assert!(wait_evidence::report(&value).is_none());
    }
    let mut value = scoped_report();
    value["clock-valid"] = 0.into();
    assert!(wait_evidence::report(&value).is_none());
    value = scoped_report();
    value["call-wall"] = 101.into();
    assert!(wait_evidence::report(&value).is_none());
    value = scoped_report();
    value["after-gc-count"] = 3.into();
    assert!(wait_evidence::report(&value).is_none());
}

#[test]
fn separate_backlog_peak_cannot_inherit_largest_gap_attribution() {
    let mut value = scoped_report();
    value["peak-wall"] = 1000.into();
    value["peak-progress-ordinal"] = 80.into();
    let result = wait_evidence::report(&value).unwrap();
    assert_eq!(result["peak_relation"], "not-associated-with-largest-gap");
    assert!(result["kernel_blocked_ns"].is_null());
    assert!(result["os_scheduling_wait_ns"].is_null());
}

fn recovery_report() -> serde_json::Value {
    fn marker(
        wall: u64,
        due: u64,
        published: u64,
        consumed: u64,
        cpu: u64,
        gc: u64,
    ) -> serde_json::Value {
        serde_json::json!({"valid":1,"wall":wall,"due":due,"published":published,"consumed":consumed,"ordinal":consumed,
            "before-valid":1,"before-begin":wall-2,"before-end":wall-1,"before-cpu":cpu,
            "before-gc_count":gc,"before-gc_wall":gc*2,"before-gc_cpu":gc,
            "after-valid":1,"after-begin":wall+1,"after-end":wall+2,"after-cpu":cpu+2,
            "after-gc_count":gc,"after-gc_wall":gc*2,"after-gc_cpu":gc})
    }
    let mut result = scoped_report();
    result["peak-wall"] = 30.into();
    result["peak-due"] = 30.into();
    result["peak-published"] = 13.into();
    result["peak-consumed"] = 13.into();
    result["peak-progress-ordinal"] = 13.into();
    result["recovery"] = serde_json::json!({"guard":3,"invalid":0,"episode":1,
        "start":marker(10,10,9,9,4,1),"peak":marker(30,30,13,13,18,2),"end":marker(40,40,40,38,24,2)});
    result
}

#[test]
fn selected_peak_uses_actual_supply_and_consumption_surplus() {
    let result = recovery_evidence::recovery_report(&recovery_report(), None).unwrap();
    assert_eq!(result["growth"]["consumption_deficit_count"], 16);
    assert_eq!(result["growth"]["cpu_gc_bounds"]["enclosing_gc_count"], 1);
    assert_eq!(result["recovery"]["consumption_surplus_count"], 15);
    assert_eq!(result["recovery"]["outstanding_end"], 2);
    assert_eq!(result["recovery"]["cpu_gc_bounds"]["enclosing_gc_count"], 0);
    assert_eq!(result["peak_source_unpublished"], 17);
    assert_eq!(result["maximum_capacity_measured"], false);
}

#[test]
fn unrelated_largest_gap_is_not_a_recovery_dependency() {
    let mut value = recovery_report();
    value["clock-valid"] = 0.into();
    value["gap-begin"] = 0.into();
    value["gap-end"] = 0.into();
    assert!(recovery_evidence::recovery_report(&value, None).is_some());
    value["recovery"]["peak"]["ordinal"] = 14.into();
    assert!(recovery_evidence::recovery_report(&value, None).is_none());
}

#[test]
fn unfinished_recovery_and_missing_cpu_cut_remain_distinct() {
    let mut value = recovery_report();
    value["recovery"]["end"]["after-valid"] = 0.into();
    let result = recovery_evidence::recovery_report(&value, None).unwrap();
    assert_eq!(result["recovery"]["consumption_surplus_count"], 15);
    assert!(result["recovery"]["cpu_gc_bounds"].is_null());
    value["recovery"]["end"]["valid"] = 0.into();
    let result = recovery_evidence::recovery_report(&value, None).unwrap();
    assert!(result["growth"].is_object());
    assert!(result["recovery"].is_null());
    value["aborted"] = 1.into();
    assert!(recovery_evidence::recovery_report(&value, None).is_none());
}

#[test]
fn rejected_episode_is_drain_after_source_stop_not_sustained_headroom() {
    let receipt: serde_json::Value = serde_json::from_str(include_str!(
        "../../../specs/scheduling/owner_peak_recovery.json"
    ))
    .unwrap();
    let source = &receipt["observed"]["scope"];
    let end = source["source-epoch"].as_u64().unwrap()
        + receipt["observed"]["metrics"]["offered_elapsed_ns"]
            .as_u64()
            .unwrap();
    let result = recovery_evidence::recovery_report(source, Some(end)).unwrap();
    assert_eq!(
        result["arrival_schedule_relation"],
        "drain-crosses-arrival-stop"
    );
    assert_eq!(result["growth"]["consumption_deficit_count"], 372_197);
    assert_eq!(result["peak_cpu_gc_bracket"]["gc_count"], 0);
    assert_eq!(result["peak_cpu_gc_bracket"]["owner_cpu_ns"], 256_375);
    assert_eq!(result["recovery"]["consumption_surplus_count"], 372_070);
    assert!(
        result["recovery"]["observed_consumption_per_second"]
            .as_f64()
            .unwrap()
            < 500_000.0
    );
    let cert = &receipt["observed"]["histogram_rank_certificate"];
    let count = cert["total"].as_u64().unwrap();
    let rank = count - count / 100;
    assert_eq!(count, 5_000_000);
    assert_eq!(rank, cert["rank99"].as_u64().unwrap());
    assert!(cert["below99"].as_u64().unwrap() < rank);
    assert!(rank <= cert["through99"].as_u64().unwrap());
}
