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

/// Receipt emitted after `cbindgen` writes a C header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCHeaderGenerationReceipt {
    /// Rust crate used as the `cbindgen` input.
    pub crate_dir: PathBuf,
    /// Generated C header path.
    pub header_file: PathBuf,
}

/// Generates a C header from a Rust crate using `cbindgen`.
///
/// # Errors
///
/// Returns an error when the output directory cannot be created, `cbindgen`
/// cannot generate the header, or the expected header file is missing after
/// generation.
pub fn write_native_c_header(
    crate_dir: &Path,
    header_file: &Path,
    include_guard: &str,
) -> Result<NativeCHeaderGenerationReceipt, String> {
    if let Some(parent) = header_file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        include_guard: Some(include_guard.to_string()),
        ..Default::default()
    };

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .map_err(|error| error.to_string())?
        .write_to_file(header_file);
    if !header_file.is_file() {
        return Err(format!(
            "native C header was not produced at {}",
            header_file.display()
        ));
    }

    Ok(NativeCHeaderGenerationReceipt {
        crate_dir: crate_dir.to_path_buf(),
        header_file: header_file.to_path_buf(),
    })
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
