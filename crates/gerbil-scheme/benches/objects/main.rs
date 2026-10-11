//! Strong-root collection and binary integer ABBA qualification, not Org parsing.
use gerbil_scheme::{CollectionKind, GerbilRuntime, RootedObjectBatch};
use std::{hint::black_box, time::Instant};
#[path = "integer.rs"]
mod integer;
mod numeric;
unsafe extern "C" {
    fn gerbil_scheme_rust_objects_usage(gc: *mut f64, os: *mut u64) -> i32;
}
fn usage() -> ([f64; 4], [u64; 4]) {
    let mut gc = [0.0; 4];
    let mut os = [0; 4];
    // SAFETY: the live Scheme owner spans all batches; disjoint complete arrays.
    assert_eq!(
        unsafe { gerbil_scheme_rust_objects_usage(gc.as_mut_ptr(), os.as_mut_ptr()) },
        0
    );
    (gc, os)
}

fn percentile(samples: &mut [u64], percent: usize) -> u64 {
    samples.sort_unstable();
    samples[(samples.len() * percent).div_ceil(100) - 1]
}
fn measure(
    name: &str,
    requests: usize,
    memory: [usize; 2],
    mut operation: impl FnMut(usize, usize),
) -> bool {
    let mut walls = [0.0; 2];
    let mut tails = [
        Vec::with_capacity(requests * 20),
        Vec::with_capacity(requests * 20),
    ];
    let mut rows = vec![0_u64; requests];
    for mode in 0..2 {
        for i in 0..1000 {
            operation(mode, i);
        }
    }
    for group in 0..10 {
        for mode in [0, 1, 1, 0] {
            let (gc_before, os_before) = usage();
            let start = Instant::now();
            for (index, sample) in rows.iter_mut().enumerate() {
                let request_start = Instant::now();
                operation(mode, index);
                *sample = u64::try_from(request_start.elapsed().as_nanos()).unwrap();
            }
            let wall = start.elapsed().as_secs_f64();
            let (gc_after, os_after) = usage();
            walls[mode] += wall;
            tails[mode].extend_from_slice(&rows);
            let p95 = percentile(&mut rows, 95);
            let p99 = percentile(&mut rows, 99);
            println!(
                "OBJECTS-BATCH family={name} requests={requests} group={group} mode={mode} wall_ms={:.6} throughput_requests_s={:.3} p95_ns={p95} p99_ns={p99} rust_reserved_bytes={} observer_reserved_bytes={}",
                wall * 1000.0,
                f64::from(u32::try_from(requests).unwrap()) / wall,
                memory[mode],
                (tails[0].capacity() + tails[1].capacity() + rows.capacity()) * 8
            );
            println!(
                "OBJECTS-COST family={name} requests={requests} group={group} mode={mode} gc_count={} gc_cpu_ms={:.6} gc_wall_ms={:.6} scheme_heap_capacity_bytes={:.0} process_cpu_ms={:.6} process_rss_high_water_bytes={} voluntary_switches={} involuntary_switches={}",
                gc_after[0] - gc_before[0],
                (gc_after[1] - gc_before[1]) * 1000.0,
                (gc_after[2] - gc_before[2]) * 1000.0,
                gc_after[3],
                std::time::Duration::from_nanos(os_after[0] - os_before[0]).as_secs_f64() * 1000.0,
                os_after[1],
                os_after[2] - os_before[2],
                os_after[3] - os_before[3]
            );
        }
    }
    let mut stats = Vec::new();
    for mode in 0..2 {
        assert_eq!(tails[mode].len(), requests * 20);
        let p95 = percentile(&mut tails[mode], 95);
        let p99 = percentile(&mut tails[mode], 99);
        println!(
            "OBJECTS-SUMMARY family={name} requests={requests} mode={mode} batches=20 total_wall_ms={:.6} throughput_requests_s={:.3} pooled_p95_ns={p95} pooled_p99_ns={p99}",
            walls[mode] * 1000.0,
            f64::from(u32::try_from(requests * 20).unwrap()) / walls[mode]
        );
        stats.push((p95, p99));
    }
    let pass = walls[1] <= walls[0] && stats[1].0 <= stats[0].0 && stats[1].1 <= stats[0].1;
    println!("OBJECTS-GATE family={name} requests={requests} admitted={pass}");
    pass
}
fn collections(runtime: &GerbilRuntime) -> bool {
    let zero = runtime
        .root_value(runtime.fixnum_value(0).unwrap())
        .unwrap();
    let false_value = runtime
        .root_value(runtime.fixture_false_value().unwrap())
        .unwrap();
    let mut admitted = true;
    for (kind, name, width) in [
        (CollectionKind::Vector, "vector", 8),
        (CollectionKind::List, "list", 8),
        (CollectionKind::Pair, "pair", 2),
    ] {
        let refs: Vec<_> = (0..width)
            .map(|i| if i % 2 == 0 { &zero } else { &false_value })
            .collect();
        let collection = runtime.collection_from_objects(kind, &refs).unwrap();
        for requests in [1000, 10_000, 100_000] {
            let mut baseline = Vec::with_capacity(width);
            let mut output = RootedObjectBatch::default();
            collection.copy_into(&mut output).unwrap();
            output.clear();
            collection.copy_into(&mut output).unwrap();
            output.clear();
            admitted &= measure(
                name,
                requests,
                [
                    baseline.capacity()
                        * std::mem::size_of::<gerbil_scheme::RootedSchemeObject<'_>>()
                        + std::mem::size_of_val(&baseline),
                    output.reserved_bytes(),
                ],
                |mode, _| {
                    if mode == 0 {
                        for index in 0..width {
                            baseline.push(collection.element(index).unwrap());
                        }
                        black_box(&baseline);
                        baseline.clear();
                    } else {
                        collection.copy_into(&mut output).unwrap();
                        black_box(&output);
                        output.clear();
                    }
                },
            );
        }
    }
    admitted
}
fn main() {
    let runtime = GerbilRuntime::initialize().unwrap();
    println!(
        "OBJECTS-METHOD owners=1 samples_per_variant=20 order=ABBA request_unit=ns wall_unit=ms memory_unit=bytes percentile=nearest_rank parser_parallelism=false workload=projection_roundtrip"
    );
    let before = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() };
    let admitted = if std::env::args().any(|arg| arg == "--numeric") {
        numeric::run(&runtime)
    } else {
        collections(&runtime) & integer::run()
    };
    assert_eq!(
        unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() },
        before
    );
    println!("OBJECTS-RESULT all_gates_pass={admitted} leaked_roots=0");
    if !admitted {
        std::process::exit(1);
    }
}
