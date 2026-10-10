// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::{path::Path, time::Instant};

use gerbil_scheme::{GERBIL_SCHEME_RUST_ABI_VERSION, GerbilRuntime};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

#[test]
fn initialized_runtime_crosses_the_live_gerbil_abi() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/unit/scenarios/native-runtime-round-trip"),
    )
    .expect("validate the runtime scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");

    assert_eq!(
        runtime.abi_version().expect("query the live ABI version"),
        GERBIL_SCHEME_RUST_ABI_VERSION,
    );
    for value in 0..10_000 {
        assert_eq!(
            runtime
                .add_i64(value, 1)
                .expect("call the exported Gerbil procedure"),
            value + 1,
        );
        assert_eq!(
            runtime
                .is_even_i64(value)
                .expect("call the exported Gerbil predicate"),
            value % 2 == 0,
        );
        assert_eq!(
            runtime
                .compare_i64(value, value + 1)
                .expect("call the exported Gerbil comparison"),
            std::cmp::Ordering::Less,
        );
    }
    assert_eq!(
        runtime
            .compare_i64(i64::MIN, i64::MIN)
            .expect("compare equal boundary values"),
        std::cmp::Ordering::Equal,
    );
    assert_eq!(
        runtime
            .compare_i64(i64::MAX, i64::MIN)
            .expect("compare opposite boundary values"),
        std::cmp::Ordering::Greater,
    );
    let elapsed = started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=native-runtime-round-trip scalar_calls=30000 elapsed_ns={}",
        elapsed.as_nanos(),
    );
    assert!(
        elapsed <= scenario.benchmark.max_total.as_duration(),
        "runtime scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        elapsed,
    );
}
