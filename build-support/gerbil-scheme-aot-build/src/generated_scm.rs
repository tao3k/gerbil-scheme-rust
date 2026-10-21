use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

const PROVENANCE_SCHEMA: &str = "gerbil-scheme-rust.generated-scm-provenance.v2";
const INPUT_PATHS: &[&str] = &[
    "build.ss",
    "gerbil.pkg",
    "scheme/runtime.ss",
    "scheme/runtime-collections.ss",
    "scheme/runtime-object-kinds.ss",
    "scheme/runtime-composites.ss",
    "scheme/utf8.ss",
    "scheme/runtime.ssi",
];

pub(crate) fn workspace_input_fingerprint(workspace: &Path) -> String {
    let mut hasher = Sha256::new();
    for relative_path in INPUT_PATHS {
        let contents = fs::read(workspace.join(relative_path))
            .unwrap_or_else(|error| panic!("read generated SCM input {relative_path}: {error}"));
        hash_framed_value(&mut hasher, relative_path.as_bytes());
        hash_framed_value(&mut hasher, &contents);
    }
    hex_digest(hasher.finalize())
}

/// External receipt: generated files remain byte-for-byte compiler output.
pub(crate) fn generated_scm_receipt(contents: &[u8], input_fingerprint: &str) -> String {
    format!(
        "{PROVENANCE_SCHEMA}\ninput-sha256={input_fingerprint}\nfile-sha256={}\n",
        sha256(contents)
    )
}

pub(crate) fn validate_generated_scm(
    contents: &[u8],
    receipt: &str,
    expected_input_fingerprint: &str,
) -> Result<(), &'static str> {
    if receipt != generated_scm_receipt(contents, expected_input_fingerprint) {
        return Err("whole-file digest or build input receipt does not match");
    }
    Ok(())
}

fn sha256(value: &[u8]) -> String {
    hex_digest(Sha256::digest(value))
}

fn hash_framed_value(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(value.len().to_le_bytes());
    hasher.update(value);
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in digest.as_ref() {
        use std::fmt::Write;
        write!(encoded, "{byte:02x}").expect("write SHA-256 digest");
    }
    encoded
}
