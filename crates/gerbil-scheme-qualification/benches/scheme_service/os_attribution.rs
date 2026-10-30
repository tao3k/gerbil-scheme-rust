//! Conservative CPU bounds from enclosing points, never interpolated run states.
use serde_json::{Value, json};
type Point = [i64; 10];
fn unsigned(row: &Point, index: usize) -> u64 {
    row[index].try_into().unwrap()
}
fn number(value: &Value, key: &str, index: usize) -> u64 {
    value[key][index].as_u64().unwrap()
}
fn request(value: &Value, owners: &[Vec<Point>; 4], index: usize, uncertainty: u64) -> Value {
    let owner = usize::try_from(number(value, "probe_owners", index)).unwrap();
    let elapsed = number(value, "probe_actual_ns", index);
    let end = value["sample_native_clock"]["at_ns"].as_u64().unwrap()
        + u64::try_from(index).unwrap() * 500_000
        + number(value, "probe_scheduled_ns", index);
    let start = end.checked_sub(elapsed).unwrap();
    let before = owners[owner]
        .iter()
        .rev()
        .find(|row| unsigned(row, 2) <= start.saturating_sub(uncertainty));
    let after = owners[owner]
        .iter()
        .find(|row| unsigned(row, 1) >= end + uncertainty);
    let inside: Vec<_> = owners[owner]
        .iter()
        .filter(|row| {
            unsigned(row, 1) >= start + uncertainty
                && unsigned(row, 2) <= end.saturating_sub(uncertainty)
        })
        .collect();
    let bounds = before.zip(after).map(|(before, after)| {
        // Two separately quantized user/system clocks; allow one us per clock.
        let cpu_upper = unsigned(after, 4).checked_sub(unsigned(before, 4)).unwrap() + 2000;
        json!({"cpu_upper_ns":cpu_upper,
            "off_cpu_lower_ns":elapsed.saturating_sub(cpu_upper),
            "outer_start_ns":before[1],"outer_end_ns":after[2]})
    });
    json!({"probe":index,"owner":owner,"start_ns":start,"end_ns":end,
        "actual_ns":elapsed,"bounds":bounds,"state_points":inside.len(),
        "runnable_or_running_points":inside.iter().filter(|row| row[3]==1).count(),
        "waiting_points":inside.iter().filter(|row| matches!(row[3],3|4)).count(),
        "unclassified_interval_ns":elapsed})
}

pub(super) fn append(value: &mut Value) {
    let samples = &value["os_thread_samples"];
    assert_eq!(samples["schema_version"], 2);
    let rows: Vec<Point> = serde_json::from_value(samples["rows"].clone()).unwrap();
    let owners: [Vec<Point>; 4] = std::array::from_fn(|owner| {
        rows.iter()
            .copied()
            .filter(|row| row[0] == i64::try_from(owner).unwrap())
            .collect()
    });
    let mut max_gap = 0;
    for (owner, points) in owners.iter().enumerate() {
        assert_eq!(
            samples["owner_tids"][owner],
            value["kernel_clock_anchors"][owner][2]
        );
        assert!(!points.is_empty());
        for pair in points.windows(2) {
            assert!(pair[1][4] >= pair[0][4], "owner CPU regressed");
            max_gap = max_gap.max(
                unsigned(&pair[1], 1)
                    .checked_sub(unsigned(&pair[0], 2))
                    .unwrap(),
            );
        }
    }
    let uncertainty = value["sample_native_clock"]["uncertainty_ns"]
        .as_u64()
        .unwrap()
        + value["kernel_clock_resolution_ns"].as_u64().unwrap();
    let requests: Vec<_> = (0..value["probe_actual_ns"].as_array().unwrap().len())
        .map(|index| request(value, &owners, index, uncertainty))
        .collect();
    value["os_request_attribution"] = json!({"clock_uncertainty_ns":uncertainty,
        "max_sampler_gap_ns":max_gap,
        "max_query_ns":rows.iter().map(|row|row[2]-row[1]).max(),
        "exact_state_intervals":false,"requests":requests});
}
