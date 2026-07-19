// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Public C header drift validation without choosing a generator as source truth.

use std::path::{Path, PathBuf};

/// Receipt comparing an authoritative public header with a generated candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCHeaderDriftReceipt {
    /// Authoritative header path.
    pub expected_header: PathBuf,
    /// Candidate header path.
    pub actual_header: PathBuf,
    /// Whether both files contain identical bytes.
    pub matched: bool,
}

/// Compares two C headers byte-for-byte and returns a stable receipt.
///
/// # Errors
///
/// Returns an I/O error when either header cannot be read.
pub fn validate_native_c_header(
    expected_header: &Path,
    actual_header: &Path,
) -> std::io::Result<NativeCHeaderDriftReceipt> {
    let expected = std::fs::read(expected_header)?;
    let actual = std::fs::read(actual_header)?;
    Ok(NativeCHeaderDriftReceipt {
        expected_header: expected_header.to_path_buf(),
        actual_header: actual_header.to_path_buf(),
        matched: expected == actual,
    })
}
