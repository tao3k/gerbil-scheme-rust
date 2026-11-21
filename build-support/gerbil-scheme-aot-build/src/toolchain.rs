// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Native C compiler discovery through the standard `cc` environment contract.

use std::path::PathBuf;

/// C compiler selected for Gerbil AOT support objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCCompilerTool {
    /// Compiler executable selected by `cc`.
    pub program: PathBuf,
}

/// Discovers the target C compiler without compiling a source file.
///
/// # Errors
///
/// Returns an error when `cc` cannot resolve a compiler for the active target.
pub fn discover_native_c_compiler() -> Result<NativeCCompilerTool, String> {
    cc::Build::new()
        .try_get_compiler()
        .map(|tool| NativeCCompilerTool {
            program: tool.path().to_path_buf(),
        })
        .map_err(|error| error.to_string())
}
