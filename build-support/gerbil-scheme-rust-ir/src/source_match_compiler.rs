//! AOT lowering for a Scheme-owned source match traversal.

use proc_macro2::{Span, TokenStream};
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

/// Character-boundary traversal selected by the source matcher IR.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceScanIr {
    Utf8CharacterBoundaries,
}

/// Candidate comparison selected by the source matcher IR.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCandidateIr {
    ExactTargetPrefix,
}

/// Word-boundary rules supplied by the Scheme strategy.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBoundaryIr {
    pub unicode_alphanumeric: bool,
    pub extra_word_characters: String,
}

/// Tie-breaking rule for competing targets at one source offset.
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
    let word_char = if unicode_alphanumeric {
        quote! { ch.is_alphanumeric() || #extra.contains(ch) }
    } else {
        quote! { #extra.contains(ch) }
    };
    let file = syn::parse2::<syn::File>(source_match_tokens(&name, &word_char))?;
    Ok(prettyplease::unparse(&file))
}

fn source_match_tokens(name: &syn::Ident, word_char: &TokenStream) -> TokenStream {
    let candidate_check = quote! {
        if !tail.starts_with(target) {
            continue;
        }
        let end = start + target.len();
        let after = source.get(end..)?.chars().next();
        if after.is_some_and(|ch| #word_char) {
            continue;
        }
        if best.is_none_or(|(best_end, _)| end > best_end) {
            best = Some((end, index));
        }
    };
    let scan = |candidate_loop: TokenStream| {
        quote! {
            for (relative, _) in remaining.char_indices() {
                let start = cursor + relative;
                let before = source.get(..start)?.chars().next_back();
                if before.is_some_and(|ch| #word_char) {
                    continue;
                }
                let mut best: Option<(usize, usize)> = None;
                let tail = source.get(start..)?;
                #candidate_loop
                if let Some((end, index)) = best {
                    return Some((start, end, index));
                }
            }
            None
        }
    };
    let linear_scan = scan(quote! {
        for (index, target) in targets.iter().enumerate() {
            if target.is_empty() {
                continue;
            }
            #candidate_check
        }
    });
    let indexed_scan = scan(quote! {
        for &(index, target) in &candidates[tail.as_bytes()[0] as usize] {
            #candidate_check
        }
    });
    quote! {
        pub fn #name(source: &str, cursor: usize, targets: &[String])
            -> Option<(usize, usize, usize)>
        {
            let remaining = source.get(cursor..)?;
            // Sparse target sets do not repay the per-call index construction.
            if targets.len() <= 32 {
                return { #linear_scan };
            }
            // Bucket exact-prefix candidates by their first UTF-8 byte. Keep
            // declaration order within each bucket for longest-then-first.
            let mut candidates: [Vec<(usize, &str)>; 256] =
                std::array::from_fn(|_| Vec::new());
            for (index, target) in targets.iter().enumerate() {
                if let Some(&first) = target.as_bytes().first() {
                    candidates[first as usize].push((index, target));
                }
            }
            #indexed_scan
        }
    }
}
