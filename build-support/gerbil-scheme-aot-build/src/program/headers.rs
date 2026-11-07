// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Package headers are declared compiler inputs, including cache identity.

use super::{NativeHeaderInput, ProgramArchiveObserver};
use sha2::{Digest, Sha256};
use std::fs;

pub(super) fn native_compile_options(
    inputs: &[NativeHeaderInput<'_>],
    observer: &dyn ProgramArchiveObserver,
) -> Result<String, String> {
    if inputs.is_empty() {
        return Ok("-O2".into());
    }
    let mut options = String::from("-O2");
    let mut digest = Sha256::new();
    digest.update(b"gerbil-native-package-headers.v1\0");
    for input in inputs {
        let root = fs::canonicalize(input.include_directory)
            .map_err(|error| format!("native header root: {error}"))?;
        if !root.is_dir() || input.header_files.is_empty() {
            return Err("native header root requires explicit files".into());
        }
        let text = root.to_str().ok_or("native header root must be UTF-8")?;
        // gsc delegates these options to its configured compiler shell.
        options.push_str(" -I'");
        options.push_str(&text.replace('\'', "'\\''"));
        options.push('\'');
        for header in input.header_files {
            let path = fs::canonicalize(header)
                .map_err(|error| format!("native header input: {error}"))?;
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| "native header is outside its include root")?;
            let bytes = fs::read(&path).map_err(|error| format!("read native header: {error}"))?;
            let name = relative
                .to_str()
                .ok_or("native header path must be UTF-8")?;
            digest.update((name.len() as u64).to_le_bytes());
            digest.update(name.as_bytes());
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(&bytes);
            observer.observe_source_input(&path);
        }
    }
    // The compiler options are already part of native object cache identity.
    // Bind header contents without changing generated C or reading ambient flags.
    options.push_str(&format!(
        " -DGERBIL_PACKAGE_HEADERS_{:x}=1",
        digest.finalize()
    ));
    Ok(options)
}
