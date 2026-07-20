// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::{path::Path, time::Instant};

use gerbil_scheme::{GerbilStatus, NativeError};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

#[test]
fn stable_status_codes_round_trip_without_transmute() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/status-contract"),
    )
    .expect("validate the status scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let statuses = [
        GerbilStatus::Ok,
        GerbilStatus::NullPointer,
        GerbilStatus::AbiMismatch,
        GerbilStatus::InvalidValue,
        GerbilStatus::RuntimeUnavailable,
        GerbilStatus::Panic,
        GerbilStatus::AlreadyInitialized,
        GerbilStatus::NotInitialized,
        GerbilStatus::RuntimeFinalized,
    ];

    let started = Instant::now();
    for _ in 0..10_000 {
        for status in statuses {
            assert_eq!(GerbilStatus::from_code(status.code()), Some(status));
        }
    }
    assert_eq!(GerbilStatus::from_code(-1), None);
    assert_eq!(GerbilStatus::from_code(9), None);
    let elapsed = started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=status-contract projections=90000 elapsed_ns={}",
        elapsed.as_nanos(),
    );
    assert!(
        elapsed <= scenario.benchmark.max_total.as_duration(),
        "status scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        elapsed,
    );
}

#[test]
fn native_errors_expose_known_statuses_and_preserve_unknown_codes() {
    assert_eq!(
        NativeError::Status {
            operation: "test operation",
            code: GerbilStatus::RuntimeUnavailable.code(),
        }
        .status(),
        Some(GerbilStatus::RuntimeUnavailable),
    );
    assert_eq!(
        NativeError::Status {
            operation: "future operation",
            code: 42,
        }
        .status(),
        None,
    );
    assert_eq!(
        NativeError::AbiMismatch {
            expected: 1,
            actual: 2,
        }
        .status(),
        Some(GerbilStatus::AbiMismatch),
    );
    assert_eq!(
        NativeError::IntegerOverflow {
            left: i64::MAX,
            right: 1,
        }
        .status(),
        Some(GerbilStatus::InvalidValue),
    );
}

#[test]
fn scheme_native_surface_projects_result_and_error_shapes() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/source-surface-sync"),
    )
    .expect("validate the source-surface synchronization scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let native_surface = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root from gerbil-scheme crate")
        .join("scheme/asp/native-surface.ss");
    let source = std::fs::read_to_string(&native_surface)
        .unwrap_or_else(|err| panic!("read {}: {err}", native_surface.display()));

    let started = Instant::now();
    for required in [
        "gerbil_scheme_rust_native_error_shape",
        "(name . native-error)",
        "(status . gerbil-status-code-preserving)",
        "(unknown-status-policy . preserve-code)",
        "(projection . optional-gerbil-status)",
        "(display-policy . operation-context-preserving)",
        "(error . native-error)",
        "(status-projection . optional-gerbil-status)",
    ] {
        assert!(
            source.contains(required),
            "missing Scheme native-surface result/error contract token: {required}"
        );
    }

    let elapsed = started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=source-surface-sync checked_tokens=8 elapsed_ns={}",
        elapsed.as_nanos(),
    );
    assert!(
        elapsed <= scenario.benchmark.max_total.as_duration(),
        "source-surface sync scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        elapsed,
    );
}
