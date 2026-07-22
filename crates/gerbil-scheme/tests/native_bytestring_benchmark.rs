#![cfg(feature = "native")]

use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use gerbil_scheme::{BytestringDelimiter, GerbilRuntime};
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
fn rooted_bytestring_round_trip_stays_sub_millisecond() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/bytevector-bytestring-round-trip"),
    )
    .expect("validate rooted bytestring round-trip scenario");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let total_started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");
    let fixture = runtime
        .fixture_bytevector_value()
        .expect("export bytevector fixture");
    let bytevector = fixture
        .as_bytevector()
        .into_result()
        .expect("project bytevector fixture");

    let mut encode_ns = sample_operation(|| {
        let rooted = bytevector
            .to_bytestring(BytestringDelimiter::SPACE)
            .into_result()
            .expect("root encoded bytestring");
        let text = rooted
            .to_string()
            .into_result()
            .expect("copy rooted bytestring");
        assert_eq!(black_box(text), "FF 7F 0B 01 00");
    });
    let mut decode_ns = sample_operation(|| {
        let rooted = runtime
            .bytevector_from_bytestring("FF 7F 0B 01 00", BytestringDelimiter::SPACE)
            .expect("root decoded bytevector");
        let bytes = rooted
            .to_vec()
            .into_result()
            .expect("copy rooted bytevector");
        assert_eq!(black_box(bytes), [255, 127, 11, 1, 0]);
    });

    let (encode_median_ns, encode_p95_ns) = summarize(&mut encode_ns);
    let (decode_median_ns, decode_p95_ns) = summarize(&mut decode_ns);
    let total = total_started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=bytevector-bytestring-round-trip samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} encode_median_ns={encode_median_ns} encode_p95_ns={encode_p95_ns} decode_median_ns={decode_median_ns} decode_p95_ns={decode_p95_ns} elapsed_ns={}",
        total.as_nanos(),
    );

    assert_budget(
        &scenario,
        "encode median",
        encode_median_ns,
        "median_bytevector_to_bytestring_budget",
    );
    assert_budget(
        &scenario,
        "encode p95",
        encode_p95_ns,
        "p95_bytevector_to_bytestring_budget",
    );
    assert_budget(
        &scenario,
        "decode median",
        decode_median_ns,
        "median_bytestring_to_bytevector_budget",
    );
    assert_budget(
        &scenario,
        "decode p95",
        decode_p95_ns,
        "p95_bytestring_to_bytevector_budget",
    );
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "rooted bytestring scenario exceeded {:?}: {:?}",
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
