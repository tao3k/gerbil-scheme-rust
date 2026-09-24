//! Typed source-line lookahead lowering for event algorithms.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;

use super::{EventPredicateIr, EventStatementIr};
use crate::CompileError;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct FutureSpec {
    target: String,
    stop: String,
    heading_marker: u8,
    heading_separator: u8,
    indent: bool,
    stop_at_heading: bool,
    body_key_marker: u8,
}

fn cache_ident(spec: &FutureSpec) -> Result<syn::Ident, CompileError> {
    let key = format!(
        "{:?}",
        (
            &spec.target,
            &spec.stop,
            spec.heading_marker,
            spec.heading_separator,
            spec.indent,
            spec.stop_at_heading,
            spec.body_key_marker,
        )
    );
    let digits = b"0123456789abcdef";
    let mut encoded = String::with_capacity(key.len() * 2);
    for byte in key.bytes() {
        encoded.push(char::from(digits[usize::from(byte >> 4)]));
        encoded.push(char::from(digits[usize::from(byte & 15)]));
    }
    syn::parse_str(&format!("__event_future_{encoded}")).map_err(Into::into)
}

fn collect_predicate(predicate: &EventPredicateIr, specs: &mut BTreeSet<FutureSpec>) {
    match predicate {
        EventPredicateIr::FutureLineMarkerBeforeBoundary {
            target,
            stop,
            heading_marker,
            heading_separator,
            indent,
            stop_at_heading,
            body_key_marker,
        } => {
            specs.insert(FutureSpec {
                target: target.clone(),
                stop: stop.clone(),
                heading_marker: *heading_marker,
                heading_separator: *heading_separator,
                indent: *indent,
                stop_at_heading: *stop_at_heading,
                body_key_marker: *body_key_marker,
            });
        }
        EventPredicateIr::Not { value } => collect_predicate(value, specs),
        EventPredicateIr::And { left, right } | EventPredicateIr::Or { left, right } => {
            collect_predicate(left, specs);
            collect_predicate(right, specs);
        }
        _ => {}
    }
}

fn collect_statements(statements: &[EventStatementIr], specs: &mut BTreeSet<FutureSpec>) {
    for statement in statements {
        match statement {
            EventStatementIr::SetBool { value, .. }
            | EventStatementIr::CloseFramesWhile {
                condition: value, ..
            } => collect_predicate(value, specs),
            EventStatementIr::If {
                condition,
                consequent,
                alternate,
            } => {
                collect_predicate(condition, specs);
                collect_statements(consequent, specs);
                collect_statements(alternate, specs);
            }
            EventStatementIr::ForLineBytes { body, .. } => collect_statements(body, specs),
            _ => {}
        }
    }
}

pub(super) fn compile_future_cache_declarations(
    statements: &[EventStatementIr],
) -> Result<Vec<TokenStream>, CompileError> {
    let mut specs = BTreeSet::new();
    collect_statements(statements, &mut specs);
    specs
        .iter()
        .map(|spec| {
            let name = cache_ident(spec)?;
            Ok(quote! {
                let #name: std::cell::OnceCell<Vec<(usize, bool)>> =
                    std::cell::OnceCell::new();
            })
        })
        .collect()
}

fn compile_future_candidate(indent: bool) -> TokenStream {
    if indent {
        quote! {
            let __event_indent = __event_future_line.as_bytes().iter()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();
            let __event_candidate = &__event_future_line[__event_indent..];
        }
    } else {
        quote! { let __event_candidate = __event_future_line; }
    }
}

fn compile_heading_boundary(marker: u8, separator: u8, enabled: bool) -> TokenStream {
    if !enabled {
        return quote! {};
    }
    quote! {
        let __event_heading_level = __event_future_line.as_bytes().iter()
            .take_while(|byte| **byte == #marker)
            .count();
        if __event_heading_level > 0
            && __event_future_line.as_bytes().get(__event_heading_level)
                == Some(&#separator)
        {
            __event_is_boundary = true;
        }
    }
}

fn compile_body_key_boundary(marker: u8) -> TokenStream {
    if marker == 0 {
        return quote! {};
    }
    quote! {
        let __event_body = __event_future_line.as_bytes();
        let __event_key_start = __event_body.iter()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count() + 1;
        let __event_valid_key = __event_body.get(__event_key_start - 1)
            == Some(&#marker)
            && __event_body[__event_key_start..].iter()
                .position(|byte| *byte == #marker)
                .is_some_and(|key_len| {
                    key_len > 0
                        && !__event_body[__event_key_start..__event_key_start + key_len]
                            .iter().any(u8::is_ascii_whitespace)
                        && __event_body.get(__event_key_start + key_len + 1)
                            .is_none_or(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                });
        if !__event_valid_key {
            __event_is_boundary = true;
        }
    }
}

pub(super) fn compile_future_line_marker(
    target: &str,
    stop: Option<&str>,
    heading_marker: u8,
    heading_separator: u8,
    indent: bool,
    stop_at_heading: bool,
    body_key_marker: u8,
) -> Result<TokenStream, CompileError> {
    if target.is_empty()
        || !target.is_ascii()
        || stop.is_some_and(|marker| marker.is_empty() || !marker.is_ascii())
    {
        return Err(CompileError::Schema(
            "future line marker must be nonempty ASCII".into(),
        ));
    }
    let cache = cache_ident(&FutureSpec {
        target: target.into(),
        stop: stop.unwrap_or_default().into(),
        heading_marker,
        heading_separator,
        indent,
        stop_at_heading,
        body_key_marker,
    })?;
    let target = syn::LitStr::new(target, proc_macro2::Span::call_site());
    let stop = stop.map(|value| syn::LitStr::new(value, proc_macro2::Span::call_site()));
    let stop_check = stop.map(|value| {
        quote! {
            if __event_future_matches(#value) {
                __event_is_boundary = true;
            }
        }
    });
    let candidate = compile_future_candidate(indent);
    let heading_check =
        compile_heading_boundary(heading_marker, heading_separator, stop_at_heading);
    let body_check = compile_body_key_boundary(body_key_marker);
    Ok(quote! {{
        let __event_future_index = #cache.get_or_init(|| {
            let mut __event_future_lines = Vec::new();
            let mut __event_future_cursor = 0usize;
            while __event_future_cursor < bytes.len() {
                let mut __event_future_end = __event_future_cursor;
                while __event_future_end < bytes.len()
                    && bytes[__event_future_end] != b'\n'
                    && bytes[__event_future_end] != b'\r'
                {
                    __event_future_end += 1;
                }
                if __event_future_end < bytes.len() {
                    if bytes[__event_future_end] == b'\r'
                        && bytes.get(__event_future_end + 1) == Some(&b'\n')
                    {
                        __event_future_end += 2;
                    } else {
                        __event_future_end += 1;
                    }
                }
                let __event_future_line = &source[__event_future_cursor..__event_future_end];
                #candidate
                let __event_future_matches = |marker: &str| {
                    __event_candidate
                        .get(..marker.len())
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(marker))
                        && __event_candidate.as_bytes()[marker.len()..]
                            .iter()
                            .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                };
                let mut __event_is_boundary = false;
                #heading_check
                #stop_check
                if !__event_is_boundary && !__event_future_matches(#target) {
                    #body_check
                }
                __event_future_lines.push((
                    __event_future_cursor,
                    __event_future_matches(#target),
                    __event_is_boundary,
                ));
                __event_future_cursor = __event_future_end;
            }
            let mut __event_found_after = false;
            for (_, target, boundary) in __event_future_lines.iter_mut().rev() {
                if *boundary {
                    __event_found_after = false;
                } else if *target {
                    __event_found_after = true;
                }
                *target = __event_found_after;
            }
            __event_future_lines
                .into_iter()
                .map(|(start, found, _)| (start, found))
                .collect()
        });
        let __event_future_position = __event_future_index
            .partition_point(|(line_start, _)| *line_start < end);
        __event_future_index
            .get(__event_future_position)
            .is_some_and(|(_, found)| *found)
    }})
}
