//! Typed source-line lookahead lowering for event algorithms.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;

use super::{EventPredicateIr, EventStatementIr, compile_offset};
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

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct NamedFutureSpec {
    target_prefix: String,
    target_suffix: String,
    stop: String,
    heading_marker: u8,
    heading_separator: u8,
    indent: bool,
    stop_at_heading: bool,
    ascii_case_insensitive: bool,
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
    encoded_cache_ident("__event_future_", &key)
}

fn named_cache_ident(spec: &NamedFutureSpec) -> Result<syn::Ident, CompileError> {
    let key = format!(
        "{:?}",
        (
            &spec.target_prefix,
            &spec.target_suffix,
            &spec.stop,
            spec.heading_marker,
            spec.heading_separator,
            spec.indent,
            spec.stop_at_heading,
            spec.ascii_case_insensitive,
        )
    );
    encoded_cache_ident("__event_named_future_", &key)
}

fn encoded_cache_ident(prefix: &str, key: &str) -> Result<syn::Ident, CompileError> {
    let digits = b"0123456789abcdef";
    let mut encoded = String::with_capacity(key.len() * 2);
    for byte in key.bytes() {
        encoded.push(char::from(digits[usize::from(byte >> 4)]));
        encoded.push(char::from(digits[usize::from(byte & 15)]));
    }
    syn::parse_str(&format!("{prefix}{encoded}")).map_err(Into::into)
}

fn collect_predicate(
    predicate: &EventPredicateIr,
    specs: &mut BTreeSet<FutureSpec>,
    named_specs: &mut BTreeSet<NamedFutureSpec>,
) {
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
        EventPredicateIr::FutureNamedLineMarkerBeforeBoundary {
            target_prefix,
            target_suffix,
            stop,
            heading_marker,
            heading_separator,
            indent,
            stop_at_heading,
            ascii_case_insensitive,
            ..
        } => {
            named_specs.insert(NamedFutureSpec {
                target_prefix: target_prefix.clone(),
                target_suffix: target_suffix.clone(),
                stop: stop.clone(),
                heading_marker: *heading_marker,
                heading_separator: *heading_separator,
                indent: *indent,
                stop_at_heading: *stop_at_heading,
                ascii_case_insensitive: *ascii_case_insensitive,
            });
        }
        EventPredicateIr::Not { value } => collect_predicate(value, specs, named_specs),
        EventPredicateIr::And { left, right } | EventPredicateIr::Or { left, right } => {
            collect_predicate(left, specs, named_specs);
            collect_predicate(right, specs, named_specs);
        }
        _ => {}
    }
}

fn collect_statements(
    statements: &[EventStatementIr],
    specs: &mut BTreeSet<FutureSpec>,
    named_specs: &mut BTreeSet<NamedFutureSpec>,
) {
    for statement in statements {
        match statement {
            EventStatementIr::SetBool { value, .. }
            | EventStatementIr::CloseFramesWhile {
                condition: value, ..
            } => collect_predicate(value, specs, named_specs),
            EventStatementIr::If {
                condition,
                consequent,
                alternate,
            } => {
                collect_predicate(condition, specs, named_specs);
                collect_statements(consequent, specs, named_specs);
                collect_statements(alternate, specs, named_specs);
            }
            EventStatementIr::ForLineBytes { body, .. }
            | EventStatementIr::WithSourceBounds { body, .. } => {
                collect_statements(body, specs, named_specs);
            }
            _ => {}
        }
    }
}

pub(super) fn compile_future_cache_declarations(
    phases: &[&[EventStatementIr]],
) -> Result<Vec<TokenStream>, CompileError> {
    let mut specs = BTreeSet::new();
    let mut named_specs = BTreeSet::new();
    for statements in phases {
        collect_statements(statements, &mut specs, &mut named_specs);
    }
    let mut declarations = specs
        .iter()
        .map(|spec| {
            let name = cache_ident(spec)?;
            Ok(quote! {
                let #name: std::cell::OnceCell<Vec<(usize, bool)>> =
                    std::cell::OnceCell::new();
            })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    for spec in &named_specs {
        let name = named_cache_ident(spec)?;
        declarations.push(quote! {
            let #name: std::cell::OnceCell<(
                Vec<(usize, usize)>,
                std::collections::HashMap<(usize, Vec<u8>), Vec<usize>>
            )> = std::cell::OnceCell::new();
        });
    }
    Ok(declarations)
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

fn compile_body_key_boundary(marker: u8, target: &syn::LitStr) -> TokenStream {
    if marker == 0 {
        return quote! {};
    }
    quote! {
      if !__event_is_boundary && !__event_future_matches(#target) {
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
    let body_check = compile_body_key_boundary(body_key_marker, &target);
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
                #body_check
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

pub(super) fn compile_future_predicate(
    predicate: &EventPredicateIr,
) -> Result<TokenStream, CompileError> {
    let EventPredicateIr::FutureLineMarkerBeforeBoundary {
        target,
        stop,
        heading_marker,
        heading_separator,
        indent,
        stop_at_heading,
        body_key_marker,
    } = predicate
    else {
        return Err(CompileError::Schema(
            "expected future marker predicate".into(),
        ));
    };
    compile_future_line_marker(
        target,
        (!stop.is_empty()).then_some(stop.as_str()),
        *heading_marker,
        *heading_separator,
        *indent,
        *stop_at_heading,
        *body_key_marker,
    )
}

fn compile_named_future_index(spec: &NamedFutureSpec) -> TokenStream {
    let prefix = syn::LitStr::new(&spec.target_prefix, proc_macro2::Span::call_site());
    let suffix = syn::LitStr::new(&spec.target_suffix, proc_macro2::Span::call_site());
    let stop = syn::LitStr::new(&spec.stop, proc_macro2::Span::call_site());
    let ci = spec.ascii_case_insensitive;
    let candidate = compile_future_candidate(spec.indent);
    let heading = compile_heading_boundary(
        spec.heading_marker,
        spec.heading_separator,
        spec.stop_at_heading,
    );
    let stop_check = if spec.stop.is_empty() {
        quote! {}
    } else {
        quote! { if __event_future_matches(#stop) { __event_is_boundary = true; } }
    };
    quote! {
        let mut __event_lines = Vec::new();
        let mut __event_closers = std::collections::HashMap::<
            (usize, Vec<u8>), Vec<usize>
        >::new();
        let mut __event_segment = 0usize;
        let mut __event_cursor = 0usize;
        while __event_cursor < bytes.len() {
            let mut __event_end = __event_cursor;
            while __event_end < bytes.len()
                && !matches!(bytes[__event_end], b'\r' | b'\n')
            {
                __event_end += 1;
            }
            if __event_end < bytes.len() {
                if bytes[__event_end] == b'\r' && bytes.get(__event_end + 1) == Some(&b'\n') {
                    __event_end += 2;
                } else {
                    __event_end += 1;
                }
            }
            let __event_future_line = &source[__event_cursor..__event_end];
            #candidate
            let __event_future_matches = |marker: &str| {
                __event_candidate.get(..marker.len())
                    .is_some_and(|value| value.eq_ignore_ascii_case(marker))
                    && __event_candidate.as_bytes()[marker.len()..].iter()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            };
            let mut __event_is_boundary = false;
            #heading
            #stop_check
            if __event_is_boundary { __event_segment += 1; }
            __event_lines.push((__event_cursor, __event_segment));
            if !__event_is_boundary {
                let __event_candidate_bytes = __event_candidate.as_bytes();
                let __event_trimmed_end = __event_candidate_bytes.iter()
                    .rposition(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                    .map_or(0, |index| index + 1);
                let __event_trimmed = &__event_candidate_bytes[..__event_trimmed_end];
                let __event_prefix = #prefix.as_bytes();
                let __event_suffix = #suffix.as_bytes();
                let __event_has_prefix = __event_trimmed.get(..__event_prefix.len())
                    .is_some_and(|value| if #ci {
                        value.eq_ignore_ascii_case(__event_prefix)
                    } else { value == __event_prefix });
                if __event_has_prefix {
                    let __event_rest = &__event_trimmed[__event_prefix.len()..];
                    let __event_has_suffix = __event_rest.get(
                        __event_rest.len().saturating_sub(__event_suffix.len())..
                    ).is_some_and(|value| if #ci {
                        value.eq_ignore_ascii_case(__event_suffix)
                    } else { value == __event_suffix });
                    if __event_has_suffix && __event_rest.len() > __event_suffix.len() {
                        let __event_name = &__event_rest[
                            ..__event_rest.len() - __event_suffix.len()
                        ];
                        let __event_key = if #ci {
                            __event_name.iter().map(|byte| byte.to_ascii_lowercase())
                                .collect::<Vec<_>>()
                        } else { __event_name.to_vec() };
                        __event_closers.entry((__event_segment, __event_key))
                            .or_default().push(__event_cursor);
                    }
                }
            }
            __event_cursor = __event_end;
        }
        (__event_lines, __event_closers)
    }
}

pub(super) fn compile_future_named_marker(
    predicate: &EventPredicateIr,
) -> Result<TokenStream, CompileError> {
    let EventPredicateIr::FutureNamedLineMarkerBeforeBoundary {
        name_from,
        name_until,
        target_prefix,
        target_suffix,
        stop,
        heading_marker,
        heading_separator,
        indent,
        stop_at_heading,
        ascii_case_insensitive,
    } = predicate
    else {
        return Err(CompileError::Schema("expected future named marker".into()));
    };
    if target_prefix.is_empty()
        || !target_prefix.is_ascii()
        || !target_suffix.is_ascii()
        || !stop.is_ascii()
    {
        return Err(CompileError::Schema(
            "future named line marker requires ASCII markers".into(),
        ));
    }
    let spec = NamedFutureSpec {
        target_prefix: target_prefix.into(),
        target_suffix: target_suffix.into(),
        stop: stop.into(),
        heading_marker: *heading_marker,
        heading_separator: *heading_separator,
        indent: *indent,
        stop_at_heading: *stop_at_heading,
        ascii_case_insensitive: *ascii_case_insensitive,
    };
    let cache = named_cache_ident(&spec)?;
    let index = compile_named_future_index(&spec);
    let name_from = compile_offset(name_from)?;
    let name_until = compile_offset(name_until)?;
    Ok(quote! {{
        let __event_name_from = #name_from;
        let __event_name_until = #name_until;
        bytes.get(__event_name_from..__event_name_until)
            .filter(|name| !name.is_empty())
            .is_some_and(|name| {
                let (__event_lines, __event_closers) = #cache.get_or_init(|| { #index });
                let __event_position = __event_lines
                    .partition_point(|(line_start, _)| *line_start < end);
                let __event_segment = __event_position.checked_sub(1)
                    .and_then(|position| __event_lines.get(position))
                    .map(|(_, segment)| *segment);
                __event_segment.is_some_and(|segment| {
                    let key = if #ascii_case_insensitive {
                        name.iter().map(|byte| byte.to_ascii_lowercase())
                            .collect::<Vec<_>>()
                    } else { name.to_vec() };
                    __event_closers.get(&(segment, key)).is_some_and(|positions| {
                        positions.partition_point(|position| *position < end)
                            < positions.len()
                    })
                })
            })
    }})
}
