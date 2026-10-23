//! Perturbed diagnostic only; native counters avoid Scheme statistics allocation.
unsafe extern "C" {
    fn gerbil_utf8_request_snapshot(slot: i32);
    fn gerbil_utf8_request_gc_stat(field: i32) -> f64;
    fn gerbil_utf8_os_stat(field: i32) -> f64;
}

pub(super) fn before() {
    // SAFETY: only the initialized Scheme owner calls these static counters.
    unsafe { gerbil_utf8_request_snapshot(0) };
}

pub(super) fn after(corpus: &str, jobs: usize, batch: usize, index: usize, elapsed: u64) {
    // SAFETY: same initialized owner; these reads neither allocate nor mutate VM.
    unsafe { gerbil_utf8_request_snapshot(1) };
    if elapsed < 1_000_000 {
        return;
    }
    let os = |i| unsafe { gerbil_utf8_os_stat(i) };
    let gc = |i| unsafe { gerbil_utf8_request_gc_stat(i) };
    assert!(os(4).is_finite() && os(4) >= 0.0, "thread CPU unavailable");
    println!(
        "UTF8-SPIKE corpus={corpus} jobs={jobs} batch={batch} index={index} service_ns={elapsed} thread_cpu_ms={:.6} gc_count={} gc_cpu_ms={:.6} gc_wall_ms={:.6} minor_faults={} major_faults={} voluntary_switches={} involuntary_switches={}",
        os(4) * 1000.0,
        gc(0),
        gc(1) * 1000.0,
        gc(2) * 1000.0,
        os(0),
        os(1),
        os(2),
        os(3)
    );
}
