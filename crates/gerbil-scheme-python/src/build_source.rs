//! Build-time source inventory for consumers of a pinned Cargo dependency.
use std::{io, path::PathBuf};

/// Locate the source tree that actually compiled this dependency.
///
/// This is a maintenance/build API, not a runtime resource loader. Consumers
/// must stage the required bytes into their package rather than retain this
/// path as an installed runtime dependency.
///
/// # Errors
/// Returns the filesystem error if the compiled dependency tree is unavailable.
pub fn build_source_root() -> io::Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
}

#[cfg(test)]
#[path = "../tests/unit/build_source.rs"]
mod tests;
