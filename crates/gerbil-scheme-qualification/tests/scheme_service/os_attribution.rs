//! Point evidence must retain unknown intervals and missing brackets.
use crate::os_attribution;
use serde_json::{Value, json};
fn sample(complete: bool) -> Value {
    let mut rows = Vec::new();
    for owner in 0..4 {
        rows.push([owner, 9000, 9001, 3, 1000, 0, 1, 31, 31, 63]);
        rows.push([owner, 12000, 12001, 1, 1500, 0, 1, 31, 31, 63]);
        if complete {
            rows.push([owner, 16000, 16001, 3, 2000, 0, 1, 31, 31, 63]);
        }
    }
    json!({"os_thread_samples":{"schema_version":2,"rows":rows,
        "owner_tids":[101,102,103,104]},
        "kernel_clock_anchors":[[0,0,101],[0,0,102],[0,0,103],[0,0,104]],
        "kernel_clock_resolution_ns":10,
        "sample_native_clock":{"at_ns":10000,"uncertainty_ns":10},
        "probe_owners":[0],"probe_actual_ns":[5000],"probe_scheduled_ns":[5000]})
}
#[test]
fn cpu_bounds_do_not_interpolate_runnable_points() {
    let mut value = sample(true);
    os_attribution::append(&mut value);
    let report = &value["os_request_attribution"];
    let request = &report["requests"][0];
    assert_eq!(request["bounds"]["cpu_upper_ns"], 3000);
    assert_eq!(request["bounds"]["off_cpu_lower_ns"], 2000);
    assert_eq!(request["runnable_or_running_points"], 1);
    assert_eq!(request["unclassified_interval_ns"], 5000);
    assert_eq!(report["exact_state_intervals"], false);
    assert_eq!(report["clock_uncertainty_ns"], 20);
}
#[test]
fn missing_final_point_cannot_establish_cpu_bound() {
    let mut value = sample(false);
    os_attribution::append(&mut value);
    assert!(value["os_request_attribution"]["requests"][0]["bounds"].is_null());
}
