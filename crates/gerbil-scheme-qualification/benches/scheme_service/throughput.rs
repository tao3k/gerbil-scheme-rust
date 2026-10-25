//! Explicit units and weighted elapsed-time aggregation, never inferred HTTP RPS.
use serde_json::{Value, json};
pub(super) fn counts(bulk_items: usize, bulk_messages: usize, short_messages: usize) -> Value {
    assert_eq!(bulk_items, bulk_messages * 8, "verified batch cardinality");
    json!({"bulk_items":bulk_items,"bulk_messages":bulk_messages,
        "short_messages":short_messages,"completed_messages":bulk_messages+short_messages,
        "bulk_lines":bulk_items*48,"short_lines":short_messages*4,
        "bulk_items_per_message":8,"bulk_lines_per_item":48,"short_lines_per_message":4})
}
pub(super) fn summary(samples: &[Value]) -> Value {
    let sum = |key: &str| {
        samples
            .iter()
            .map(|s| s["work_counts"][key].as_u64().unwrap())
            .sum::<u64>()
    };
    let elapsed: u64 = samples
        .iter()
        .map(|s| s["elapsed_ns"].as_u64().unwrap())
        .sum();
    let bulk_elapsed: u64 = samples
        .iter()
        .map(|s| s["bulk_elapsed_ns"].as_u64().unwrap())
        .sum();
    assert!(elapsed > 0 && bulk_elapsed > 0);
    let rate = |count: u64, ns: u64| {
        f64::from(u32::try_from(count).expect("bounded sample count"))
            / std::time::Duration::from_nanos(ns).as_secs_f64()
    };
    json!({"contract":"service-message-throughput-v1","windows":samples.len(),
        "completed_messages":sum("completed_messages"),"bulk_messages":sum("bulk_messages"),
        "short_messages":sum("short_messages"),"bulk_items":sum("bulk_items"),
        "measurement_elapsed_ns":elapsed,"native_bulk_elapsed_ns":bulk_elapsed,
        "completed_messages_per_second":rate(sum("completed_messages"),elapsed),
        "bulk_messages_per_native_second":rate(sum("bulk_messages"),bulk_elapsed),
        "bulk_items_per_native_second":rate(sum("bulk_items"),bulk_elapsed),
        "short_messages_per_measurement_second":rate(sum("short_messages"),elapsed),
        "combined_input_items_per_second":rate(sum("bulk_items")+sum("short_messages"),elapsed),
        "is_http_rps":false,"is_ffi_calls_per_second":false,
        "timing":"release through bulk and short client consumption; initialization/warmup/cleanup excluded",
        "native_bulk_timing":"release through last native bulk completion; client recovery is separate"})
}

/// Nearest-rank quantile in basis points, without fabricated missing samples.
pub(super) fn rank(values: &mut [u64], basis_points: usize) -> Option<u64> {
    assert!((1..=10_000).contains(&basis_points));
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[(values.len() * basis_points).div_ceil(10_000) - 1])
}
