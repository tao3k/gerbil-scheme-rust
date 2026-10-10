use std::{hint::black_box, path::Path, time::Instant};

use gerbil_scheme::{
    ByteOrder, BytestringDelimiter, GerbilRuntime, IntegerDecoding, IntegerEncoding, IntegerWidth,
    RootedSchemeValue, RootedSchemeValueKind,
};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

const SAMPLE_COUNT: usize = 40;
const ITERATIONS_PER_SAMPLE: usize = 25;
const MEDIAN_BUDGET_NS: u128 = 500_000;
const P95_BUDGET_NS: u128 = 900_000;

#[test]
fn rooted_scheme_value_round_trip_stays_sub_millisecond() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/rooted-scheme-value-round-trip"),
    )
    .expect("validate rooted Scheme value scenario");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let total_started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");
    let fixture = runtime
        .fixture_bytevector_value()
        .expect("export bytevector fixture");
    let borrowed = fixture
        .as_bytevector()
        .into_result()
        .expect("project bytevector fixture");
    let width = IntegerWidth::new(8).expect("eight-byte integer width");
    let encoding = IntegerEncoding::fixed(ByteOrder::Big, width);
    let decoding = IntegerDecoding::entire(ByteOrder::Big);

    let mut exact_integer_ns = sample_operation(|| {
        let value: RootedSchemeValue<'_> = runtime
            .exact_integer_from_i64(-23)
            .expect("root exact integer")
            .into();
        assert_eq!(value.kind(), RootedSchemeValueKind::ExactInteger);
        let projected = value
            .as_exact_integer()
            .expect("typed exact integer")
            .to_i64()
            .into_result()
            .expect("project exact integer");
        assert_eq!(black_box(projected), -23);
    });
    let mut bytevector_ns = sample_operation(|| {
        let value: RootedSchemeValue<'_> = runtime
            .uint_to_bytevector(0x01_02_03_04_05_06_07_08, encoding)
            .expect("root bytevector")
            .into();
        assert_eq!(value.kind(), RootedSchemeValueKind::Bytevector);
        let projected = value
            .as_bytevector()
            .expect("typed bytevector")
            .to_uint(decoding)
            .into_result()
            .expect("decode bytevector");
        assert_eq!(black_box(projected), 0x01_02_03_04_05_06_07_08);
    });
    let mut string_ns = sample_operation(|| {
        let value: RootedSchemeValue<'_> = borrowed
            .to_bytestring(BytestringDelimiter::SPACE)
            .into_result()
            .expect("root Scheme string")
            .into();
        assert_eq!(value.kind(), RootedSchemeValueKind::String);
        let projected = value
            .as_string()
            .expect("typed Scheme string")
            .to_string()
            .into_result()
            .expect("copy Scheme string");
        assert_eq!(black_box(projected), "FF 7F 0B 01 00");
    });

    let (exact_integer_median_ns, exact_integer_p95_ns) = summarize(&mut exact_integer_ns);
    let (bytevector_median_ns, bytevector_p95_ns) = summarize(&mut bytevector_ns);
    let (string_median_ns, string_p95_ns) = summarize(&mut string_ns);
    let total = total_started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=rooted-scheme-value-round-trip samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE} exact_integer_median_ns={exact_integer_median_ns} exact_integer_p95_ns={exact_integer_p95_ns} bytevector_median_ns={bytevector_median_ns} bytevector_p95_ns={bytevector_p95_ns} string_median_ns={string_median_ns} string_p95_ns={string_p95_ns} elapsed_ns={}",
        total.as_nanos(),
    );

    for (label, median_ns, p95_ns) in [
        (
            "exact integer",
            exact_integer_median_ns,
            exact_integer_p95_ns,
        ),
        ("bytevector", bytevector_median_ns, bytevector_p95_ns),
        ("string", string_median_ns, string_p95_ns),
    ] {
        assert!(
            median_ns <= MEDIAN_BUDGET_NS,
            "{label} median {median_ns}ns exceeded {MEDIAN_BUDGET_NS}ns",
        );
        assert!(
            p95_ns <= P95_BUDGET_NS,
            "{label} p95 {p95_ns}ns exceeded {P95_BUDGET_NS}ns",
        );
    }
    assert!(
        total <= scenario.benchmark.max_total.as_duration(),
        "rooted Scheme value scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        total,
    );
}

fn sample_operation(mut operation: impl FnMut()) -> Vec<u128> {
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            operation();
        }
        samples.push(started.elapsed().as_nanos() / ITERATIONS_PER_SAMPLE as u128);
    }
    samples
}

fn summarize(samples: &mut [u128]) -> (u128, u128) {
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p95_index = (samples.len() * 95).div_ceil(100).saturating_sub(1);
    (median, samples[p95_index])
}
