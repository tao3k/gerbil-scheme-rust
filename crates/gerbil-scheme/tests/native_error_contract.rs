// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::NativeError;
use gerbil_scheme_sys::GerbilStatus;

#[test]
fn invalid_comparison_result_fails_closed_as_an_invalid_value() {
    let error = NativeError::InvalidComparisonResult { code: 7 };

    assert_eq!(error.status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(error.to_string(), "invalid Gerbil i64 comparison result 7");
}
