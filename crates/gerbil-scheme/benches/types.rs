//! Matched whole-scalar FFI qualification; no parsing or queue concurrency claim.
use gerbil_scheme::{GerbilRuntime, GerbilValue, ScalarBatch, SchemeScalar};
use std::{hint::black_box, time::Instant};

fn scalar(value: GerbilValue<'_>, kind: usize) -> SchemeScalar {
    match kind {
        0 => SchemeScalar::Boolean(value.as_boolean().into_result().unwrap()),
        1 => SchemeScalar::Fixnum(value.as_fixnum().into_result().unwrap()),
        2 => SchemeScalar::Char(value.as_char().into_result().unwrap()),
        3 => SchemeScalar::Flonum(value.as_flonum().into_result().unwrap()),
        _ => unreachable!(),
    }
}

fn equal(left: &[SchemeScalar], right: &[SchemeScalar]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| match (a, b) {
            (SchemeScalar::Flonum(a), SchemeScalar::Flonum(b)) => a.to_bits() == b.to_bits(),
            _ => a == b,
        })
}

fn percentile(samples: &mut [u64], percent: usize) -> u64 {
    samples.sort_unstable();
    samples[(samples.len() * percent).div_ceil(100) - 1]
}

fn fixtures(runtime: &GerbilRuntime) -> [Vec<GerbilValue<'_>>; 4] {
    [
        vec![
            runtime.fixture_true_value().unwrap(),
            runtime.fixture_false_value().unwrap(),
        ],
        vec![runtime.fixture_fixnum_value().unwrap()],
        vec![
            runtime.fixture_char_ascii_value().unwrap(),
            runtime.fixture_char_bmp_value().unwrap(),
            runtime.fixture_char_non_bmp_value().unwrap(),
        ],
        vec![
            runtime.fixture_flonum_finite_value().unwrap(),
            runtime.fixture_flonum_nan_value().unwrap(),
            runtime.fixture_flonum_pos_inf_value().unwrap(),
            runtime.fixture_flonum_neg_inf_value().unwrap(),
            runtime.fixture_flonum_neg_zero_value().unwrap(),
        ],
    ]
}

fn main() {
    let runtime = GerbilRuntime::initialize().unwrap();
    let fixtures = fixtures(&runtime);
    println!(
        "TYPES-METHOD samples_per_variant=20 order=ABBA owners=1 values_per_request=8 percentiles=nearest_rank request_unit=ns wall_unit=ms memory_unit=bytes scope=scalar_projection parser_parallelism=false"
    );
    let mut admitted = true;
    for family in 0..5 {
        let name = ["boolean", "fixnum", "char", "flonum", "mixed"][family];
        let kinds: Vec<_> = (0..8)
            .map(|i| if family == 4 { i % 4 } else { family })
            .collect();
        let rows: Vec<Vec<_>> = (0..5)
            .map(|row| {
                kinds
                    .iter()
                    .enumerate()
                    .map(|(column, &kind)| fixtures[kind][(row + column) % fixtures[kind].len()])
                    .collect()
            })
            .collect();
        let expected: Vec<Vec<_>> = rows
            .iter()
            .map(|values| {
                values
                    .iter()
                    .zip(&kinds)
                    .map(|(&v, &k)| scalar(v, k))
                    .collect()
            })
            .collect();
        for requests in [1_000, 10_000, 100_000] {
            let mut batch = ScalarBatch::default();
            let mut output = Vec::with_capacity(8);
            for (values, expected) in rows.iter().zip(&expected) {
                batch.project(&runtime, values, &mut output).unwrap();
                assert!(equal(&output, expected));
            }
            let mut timings = [Vec::with_capacity(20), Vec::with_capacity(20)];
            let mut tails = [
                Vec::with_capacity(requests * 20),
                Vec::with_capacity(requests * 20),
            ];
            let mut scratch = vec![0_u64; requests];
            for group in 0..10 {
                for mode in [0, 1, 1, 0] {
                    let start = Instant::now();
                    for (index, sample) in scratch.iter_mut().enumerate() {
                        let values = &rows[index % rows.len()];
                        let request_start = Instant::now();
                        if mode == 0 {
                            output.clear();
                            output.extend(values.iter().zip(&kinds).map(|(&v, &k)| scalar(v, k)));
                        } else {
                            batch.project(&runtime, values, &mut output).unwrap();
                        }
                        black_box(&output);
                        *sample = u64::try_from(request_start.elapsed().as_nanos()).unwrap();
                    }
                    let wall = start.elapsed().as_secs_f64();
                    assert!(equal(&output, &expected[(requests - 1) % rows.len()]));
                    timings[mode].push(wall);
                    tails[mode].extend_from_slice(&scratch);
                    let p95 = percentile(&mut scratch, 95);
                    let p99 = percentile(&mut scratch, 99);
                    println!(
                        "TYPES-BATCH family={name} requests={requests} group={group} mode={mode} wall_ms={:.6} throughput_requests_s={:.3} p95_ns={p95} p99_ns={p99} scratch_reserved_bytes={} output_reserved_bytes={} observer_reserved_bytes={}",
                        wall * 1000.0,
                        f64::from(u32::try_from(requests).unwrap()) / wall,
                        if mode == 0 { 0 } else { batch.reserved_bytes() },
                        output.capacity() * std::mem::size_of::<SchemeScalar>(),
                        (tails[0].capacity() + tails[1].capacity() + scratch.capacity()) * 8
                    );
                }
            }
            let mut stats = Vec::new();
            for mode in 0..2 {
                assert_eq!(timings[mode].len(), 20);
                let wall: f64 = timings[mode].iter().sum();
                let p95 = percentile(&mut tails[mode], 95);
                let p99 = percentile(&mut tails[mode], 99);
                println!(
                    "TYPES-SUMMARY family={name} requests={requests} mode={mode} batches=20 total_wall_ms={:.6} throughput_requests_s={:.3} pooled_p95_ns={p95} pooled_p99_ns={p99}",
                    wall * 1000.0,
                    f64::from(u32::try_from(requests * 20).unwrap()) / wall
                );
                stats.push((wall, p95, p99));
            }
            let pass =
                stats[1].0 <= stats[0].0 && stats[1].1 <= stats[0].1 && stats[1].2 <= stats[0].2;
            admitted &= pass;
            println!("TYPES-GATE family={name} requests={requests} admitted={pass}");
        }
    }
    println!("TYPES-RESULT all_gates_pass={admitted}");
    if !admitted {
        std::process::exit(1);
    }
}
