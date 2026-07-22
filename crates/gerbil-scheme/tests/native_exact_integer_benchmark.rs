#![cfg(feature = "native")]

use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use gerbil_scheme::{ExactIntegerTarget, GerbilRuntime, NativeError};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

const SAMPLE_COUNT: usize = 40;
const ITERATIONS_PER_SAMPLE: usize = 25;
type Samples = [u128; SAMPLE_COUNT];

fn sample_operation(mut operation: impl FnMut()) -> Samples {
    let mut samples = [0_u128; SAMPLE_COUNT];
    for sample in &mut samples {
        let started = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            operation();
        }
        *sample = started.elapsed().as_nanos() / ITERATIONS_PER_SAMPLE as u128;
    }
    samples
}

fn summarize(samples: &mut Samples) -> (u128, u128) {
    samples.sort_unstable();
    (u128::midpoint(samples[19], samples[20]), samples[37])
}

#[test]
fn exact_integer_projection_round_trip_stays_sub_millisecond() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/exact-integer-projection-round-trip"),
    )
    .expect("validate exact integer projection scenario");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let total_started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");

    let signed_value = i64::MIN;
    let mut signed_ns = sample_operation(|| {
        let rooted = runtime
            .exact_integer_from_i64(black_box(signed_value))
            .expect("root signed exact integer");
        let decoded = rooted
            .to_i64()
            .into_result()
            .expect("project signed exact integer");
        assert_eq!(black_box(decoded), signed_value);
    });

    let unsigned_value = u64::MAX;
    let mut unsigned_ns = sample_operation(|| {
        let rooted = runtime
            .exact_integer_from_u64(black_box(unsigned_value))
            .expect("root unsigned exact integer");
        let decoded = rooted
            .to_u64()
            .into_result()
            .expect("project unsigned exact integer");
        assert_eq!(black_box(decoded), unsigned_value);
    });

    let oversized = runtime
        .fixture_exact_integer_large_positive_value()
        .expect("export oversized bignum")
        .as_exact_integer()
        .into_result()
        .expect("project oversized bignum handle");
    let mut rejection_ns = sample_operation(|| {
        assert!(matches!(
            black_box(oversized).to_u64().into_result(),
            Err(NativeError::ExactIntegerOutOfRange {
                target: ExactIntegerTarget::U64
            })
        ));
    });

    let (signed_median_ns, signed_p95_ns) = summarize(&mut signed_ns);
    let (unsigned_median_ns, unsigned_p95_ns) = summarize(&mut unsigned_ns);
    let (rejection_median_ns, rejection_p95_ns) = summarize(&mut rejection_ns);
    let total = total_started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=exact-integer-projection-round-trip samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} signed_median_ns={signed_median_ns} signed_p95_ns={signed_p95_ns} unsigned_median_ns={unsigned_median_ns} unsigned_p95_ns={unsigned_p95_ns} rejection_median_ns={rejection_median_ns} rejection_p95_ns={rejection_p95_ns} elapsed_ns={}",
        total.as_nanos(),
    );

    assert_budget(
        &scenario,
        "signed median",
        signed_median_ns,
        "median_signed_round_trip_budget",
    );
    assert_budget(
        &scenario,
        "signed p95",
        signed_p95_ns,
        "p95_signed_round_trip_budget",
    );
    assert_budget(
        &scenario,
        "unsigned median",
        unsigned_median_ns,
        "median_unsigned_round_trip_budget",
    );
    assert_budget(
        &scenario,
        "unsigned p95",
        unsigned_p95_ns,
        "p95_unsigned_round_trip_budget",
    );
    assert_budget(
        &scenario,
        "rejection median",
        rejection_median_ns,
        "median_out_of_range_budget",
    );
    assert_budget(
        &scenario,
        "rejection p95",
        rejection_p95_ns,
        "p95_out_of_range_budget",
    );
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "exact integer scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        total,
    );
}

fn assert_budget(
    scenario: &rust_lang_project_harness::RustScenarioBenchmarkReceipt,
    name: &str,
    observed_ns: u128,
    budget_name: &str,
) {
    let budget = scenario
        .benchmark
        .observed_timings
        .get(budget_name)
        .unwrap_or_else(|| panic!("scenario declares {budget_name}"))
        .as_duration();
    assert!(
        observed_ns <= budget.as_nanos(),
        "{name} exceeded {budget:?}: {observed_ns}ns",
    );
}
