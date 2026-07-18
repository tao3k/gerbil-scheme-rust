// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{GerbilStatus, NativeError};

#[test]
fn stable_status_codes_round_trip_without_transmute() {
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

    for status in statuses {
        assert_eq!(GerbilStatus::from_code(status.code()), Some(status));
    }
    assert_eq!(GerbilStatus::from_code(-1), None);
    assert_eq!(GerbilStatus::from_code(9), None);
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
