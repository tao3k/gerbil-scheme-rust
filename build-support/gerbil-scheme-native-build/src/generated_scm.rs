use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

const PROVENANCE_SCHEMA: &str = "gerbil-scheme-rust.generated-scm-provenance.v1";
const INPUT_PATHS: &[&str] = &[
    "build.ss",
    "gerbil.pkg",
    "scheme/native.ss",
    "scheme/native.ssi",
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

pub(crate) fn stamp_generated_scm(body: &str, input_fingerprint: &str) -> String {
    let body_fingerprint = sha256(body.as_bytes());
    format!(
        ";; {PROVENANCE_SCHEMA} input-sha256={input_fingerprint} \
         body-sha256={body_fingerprint}\n{body}"
    )
}

pub(crate) fn validate_generated_scm(
    tracked: &str,
    expected_input_fingerprint: &str,
) -> Result<(), &'static str> {
    let (header, body) = tracked
        .split_once('\n')
        .ok_or("missing provenance header separator")?;
    let provenance = parse_provenance(header)?;
    if provenance.input_fingerprint != expected_input_fingerprint {
        return Err("input fingerprint does not match current Scheme build inputs");
    }
    if provenance.body_fingerprint != sha256(body.as_bytes()) {
        return Err("body fingerprint does not match committed SCM contents");
    }
    Ok(())
}

struct GeneratedScmProvenance<'a> {
    input_fingerprint: &'a str,
    body_fingerprint: &'a str,
}

fn parse_provenance(header: &str) -> Result<GeneratedScmProvenance<'_>, &'static str> {
    let mut fields = header.split_ascii_whitespace();
    if fields.next() != Some(";;") || fields.next() != Some(PROVENANCE_SCHEMA) {
        return Err("missing generated SCM provenance schema");
    }
    let input_fingerprint = fingerprint_field(fields.next(), "input-sha256=")?;
    let body_fingerprint = fingerprint_field(fields.next(), "body-sha256=")?;
    if fields.next().is_some() {
        return Err("unexpected generated SCM provenance fields");
    }
    Ok(GeneratedScmProvenance {
        input_fingerprint,
        body_fingerprint,
    })
}

fn fingerprint_field<'a>(field: Option<&'a str>, prefix: &str) -> Result<&'a str, &'static str> {
    let fingerprint = field
        .and_then(|field| field.strip_prefix(prefix))
        .ok_or("missing generated SCM fingerprint field")?;
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("generated SCM fingerprint is not lowercase SHA-256");
    }
    Ok(fingerprint)
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
