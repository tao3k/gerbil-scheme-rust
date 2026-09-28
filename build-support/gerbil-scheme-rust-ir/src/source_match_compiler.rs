//! AOT lowering for a Scheme-owned source match traversal.

use proc_macro2::Span;
use quote::quote;
use serde::Deserialize;

use crate::CompileError;

/// Closed wire contract for a source-backed, document-local match algorithm.
pub const SOURCE_MATCH_IR_SCHEMA: &str = "gerbil-scheme-rust.source-match-ir.v1";

/// Scheme selects traversal, boundary, and winner semantics; Rust is generated.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMatchIr {
    pub schema: String,
    pub name: String,
    pub scan: SourceScanIr,
    pub candidate: SourceCandidateIr,
    pub boundary: SourceBoundaryIr,
    pub winner: SourceWinnerIr,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceScanIr {
    Utf8CharacterBoundaries,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCandidateIr {
    ExactTargetPrefix,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBoundaryIr {
    pub unicode_alphanumeric: bool,
    pub extra_word_characters: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWinnerIr {
    LongestThenFirst,
}

/// Compile a versioned Scheme source matcher into a typed Rust function.
///
/// # Errors
/// Rejects an unknown schema, malformed fields, or invalid Rust syntax.
pub fn compile_source_match_json(input: &str) -> Result<String, CompileError> {
    let algorithm: SourceMatchIr = serde_json::from_str(input).map_err(CompileError::Json)?;
    compile_source_match(&algorithm)
}

/// Lower the selected traversal into Rust syntax without raw-code IR fields.
///
/// # Errors
/// Rejects an unknown schema or invalid function name.
pub fn compile_source_match(algorithm: &SourceMatchIr) -> Result<String, CompileError> {
    if algorithm.schema != SOURCE_MATCH_IR_SCHEMA {
        return Err(CompileError::Schema(algorithm.schema.clone()));
    }
    let name = syn::parse_str::<syn::Ident>(&algorithm.name)?;
    let SourceScanIr::Utf8CharacterBoundaries = algorithm.scan;
    let SourceCandidateIr::ExactTargetPrefix = algorithm.candidate;
    let SourceWinnerIr::LongestThenFirst = algorithm.winner;
    let unicode_alphanumeric = algorithm.boundary.unicode_alphanumeric;
    let extra = syn::LitStr::new(&algorithm.boundary.extra_word_characters, Span::call_site());
    let tokens = quote! {
        pub fn #name(source: &str, cursor: usize, targets: &[String])
            -> Option<(usize, usize, usize)>
        {
            let remaining = source.get(cursor..)?;
            for (relative, _) in remaining.char_indices() {
                let start = cursor + relative;
                let before = source.get(..start)?.chars().next_back();
                if before.is_some_and(|ch| (#unicode_alphanumeric && ch.is_alphanumeric())
                    || #extra.contains(ch)) {
                    continue;
                }
                let mut best: Option<(usize, usize)> = None;
                for (index, target) in targets.iter().enumerate() {
                    if target.is_empty() || !source.get(start..)?.starts_with(target) {
                        continue;
                    }
                    let end = start + target.len();
                    let after = source.get(end..)?.chars().next();
                    if after.is_some_and(|ch| (#unicode_alphanumeric && ch.is_alphanumeric())
                        || #extra.contains(ch)) {
                        continue;
                    }
                    if best.is_none_or(|(best_end, _)| end > best_end) {
                        best = Some((end, index));
                    }
                }
                if let Some((end, index)) = best {
                    return Some((start, end, index));
                }
            }
            None
        }
    };
    let file = syn::parse2::<syn::File>(tokens)?;
    Ok(prettyplease::unparse(&file))
}
