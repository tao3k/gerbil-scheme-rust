// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::NativeError;
use gerbil_scheme_sys::GerbilStatus;

/// Validate all bytes before transferring the same owned allocation to String.
/// No Scheme provenance, snapshot or input identity bypasses validation.
pub(super) fn validated_string(bytes: Vec<u8>) -> Result<String, NativeError> {
    simdutf8::basic::from_utf8(&bytes).map_err(|_| NativeError::Status {
        operation: "gerbil_scheme_rust_root_string_to_utf8",
        code: GerbilStatus::InvalidValue as i32,
    })?;
    // SAFETY: the SIMD validator checked the entire vector above. Exclusive
    // ownership prevents mutation between validation and this no-copy handoff.
    // Rust's String contract needs valid UTF-8, not a second identical scan.
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

#[cfg(test)]
#[path = "../../tests/unit/utf8_validation.rs"]
mod tests;
