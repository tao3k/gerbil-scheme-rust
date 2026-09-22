#![cfg(feature = "native")]

use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use gerbil_scheme::{ByteOrder, GerbilRuntime, IntegerDecoding, IntegerEncoding, IntegerWidth};
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
fn integer_bytevector_round_trip_stays_sub_millisecond() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/integer-bytevector-round-trip"),
    )
    .expect("validate integer bytevector round-trip scenario");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let total_started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");
    let width = IntegerWidth::new(8).expect("eight-byte machine integer width");
    let encoding = IntegerEncoding::fixed(ByteOrder::Big, width);
    let decoding = IntegerDecoding::entire(ByteOrder::Big);

    let unsigned_value = 0x01_02_03_04_05_06_07_08_u64;
    let mut unsigned_ns = sample_operation(|| {
        let rooted = runtime
            .uint_to_bytevector(unsigned_value, encoding)
            .expect("encode rooted unsigned integer");
        let decoded = rooted
            .to_uint(decoding)
            .into_result()
            .expect("decode rooted unsigned integer");
        assert_eq!(black_box(decoded), unsigned_value);
    });

    let signed_value = -0x01_02_03_04_05_06_07_i64;
    let mut signed_ns = sample_operation(|| {
        let rooted = runtime
            .sint_to_bytevector(signed_value, encoding)
            .expect("encode rooted signed integer");
        let decoded = rooted
            .to_sint(decoding)
            .into_result()
            .expect("decode rooted signed integer");
        assert_eq!(black_box(decoded), signed_value);
    });

    let (unsigned_median_ns, unsigned_p95_ns) = summarize(&mut unsigned_ns);
    let (signed_median_ns, signed_p95_ns) = summarize(&mut signed_ns);
    let total = total_started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=integer-bytevector-round-trip samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} unsigned_median_ns={unsigned_median_ns} unsigned_p95_ns={unsigned_p95_ns} signed_median_ns={signed_median_ns} signed_p95_ns={signed_p95_ns} elapsed_ns={}",
        total.as_nanos(),
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
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "integer bytevector scenario exceeded {:?}: {:?}",
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
