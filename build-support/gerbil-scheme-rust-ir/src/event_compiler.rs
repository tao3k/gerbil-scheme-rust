//! Bounded stateful event procedures lowered from Scheme-owned parser IR.
//!
//! This backend owns no language rule: it validates a closed event vocabulary
//! and constructs Rust syntax. The Scheme frontend owns every predicate and
//! transition supplied through the versioned IR.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;

use crate::CompileError;

#[path = "event_future_compiler.rs"]
mod event_future_compiler;
#[path = "event_list_compiler.rs"]
mod event_list_compiler;
#[path = "event_marker_collector.rs"]
mod event_marker_collector;
use event_future_compiler::{
    compile_future_cache_declarations, compile_future_named_marker, compile_future_predicate,
};
use event_list_compiler::{
    compile_close_all_frames, compile_close_frames_while, compile_scan_list_marker,
};
use event_marker_collector::collect_line_markers;

#[path = "event_ir_types.rs"]
mod event_ir_types;
pub use event_ir_types::{
    EVENT_FUNCTION_IR_SCHEMA, EventBoundaryIr, EventComputedOffsetIr, EventFunctionIr,
    EventHelperIr, EventListMarkerIr, EventOffsetIr, EventPredicateIr, EventStatementIr,
    EventUsizeIr,
};
/// Compile a versioned Scheme event IR document into a Rust event function.
///
/// # Errors
/// Rejects unknown wire forms, identifiers, digests, and generated syntax.
pub fn compile_event_function_json(input: &str) -> Result<String, CompileError> {
    let function: EventFunctionIr = serde_json::from_str(input).map_err(CompileError::Json)?;
    compile_event_function(&function)
}

/// Construct and validate Rust tokens for a Scheme-owned event procedure.
///
/// # Errors
/// Rejects unknown schemas, malformed identifiers or digests, and invalid syntax.
pub fn compile_event_function(function: &EventFunctionIr) -> Result<String, CompileError> {
    validate_event_function(function)?;
    let name = syn::parse_str::<syn::Ident>(&function.name)?;
    let root = function.root_kind;
    let digest = syn::LitStr::new(&function.parser_digest, proc_macro2::Span::call_site());
    let initial = compile_statements(&function.initial)?;
    let line = compile_statements(&function.line)?;
    let finish = compile_statements(&function.finish)?;
    let helpers = compile_event_helpers(function)?;
    let cached_markers = compile_marker_cache(&function.line)?;
    let future_caches = compile_future_cache_declarations(&[&function.line, &function.finish])?;
    let tokens = quote! {
        pub const PARSER_DIGEST: &str = #digest;

        pub fn #name(source: &str) -> Vec<TreeEvent> {
            #(#helpers)*
            let bytes = source.as_bytes();
            let mut events = Vec::with_capacity(bytes.len() / 16 + 2);
            events.push(TreeEvent::StartNode(#root));
            #initial
            #(#future_caches)*
            let mut start = 0usize;
            while start < bytes.len() {
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
                    end += 1;
                }
                if end < bytes.len() {
                    if bytes[end] == b'\r' && bytes.get(end + 1) == Some(&b'\n') {
                        end += 2;
                    } else {
                        end += 1;
                    }
                }
                let line = &source[start..end];
                #(#cached_markers)*
                #line
                start = end;
            }
            #finish
            events.push(TreeEvent::FinishNode);
            events
        }
    };
    let file = syn::parse2::<syn::File>(tokens)?;
    Ok(prettyplease::unparse(&file))
}

fn validate_event_function(function: &EventFunctionIr) -> Result<(), CompileError> {
    if function.schema != EVENT_FUNCTION_IR_SCHEMA {
        return Err(CompileError::Schema(function.schema.clone()));
    }
    if !function.parser_digest.starts_with("sha256:")
        || function.parser_digest.len() != 71
        || !function.parser_digest[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(CompileError::Schema(function.parser_digest.clone()));
    }
    Ok(())
}

fn compile_event_helpers(function: &EventFunctionIr) -> Result<Vec<TokenStream>, CompileError> {
    let helper_names = function
        .helpers
        .iter()
        .map(|helper| helper.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if helper_names.len() != function.helpers.len() {
        return Err(CompileError::Schema("duplicate event helper name".into()));
    }
    validate_helper_calls(&function.line, &helper_names, true)?;
    validate_helper_calls(&function.finish, &helper_names, true)?;
    function
        .helpers
        .iter()
        .map(|helper| {
            validate_helper_calls(&helper.body, &helper_names, false)?;
            compile_event_helper(helper)
        })
        .collect::<Result<Vec<_>, _>>()
}

fn compile_marker_cache(statements: &[EventStatementIr]) -> Result<Vec<TokenStream>, CompileError> {
    let mut markers = BTreeSet::new();
    collect_line_markers(statements, &mut markers);
    markers
        .into_iter()
        .map(|(marker, separator)| {
            let name = marker_name(marker, separator)?;
            Ok(quote! {
                let #name = {
                    let run = line.as_bytes().iter().take_while(|byte| **byte == #marker).count();
                    if run > 0 && line.as_bytes().get(run) == Some(&#separator) {
                        run
                    } else {
                        0
                    }
                };
            })
        })
        .collect()
}

fn marker_name(marker: u8, separator: u8) -> Result<syn::Ident, CompileError> {
    Ok(syn::parse_str(&format!(
        "__event_marker_{marker}_{separator}"
    ))?)
}

fn line_index_name(name: &str) -> Result<syn::Ident, CompileError> {
    let name = syn::parse_str::<syn::Ident>(name)?;
    Ok(syn::parse_str(&format!("__event_index_{name}"))?)
}

fn compile_statements(statements: &[EventStatementIr]) -> Result<TokenStream, CompileError> {
    let tokens = statements
        .iter()
        .map(compile_statement)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { #(#tokens)* })
}

fn compile_statement(statement: &EventStatementIr) -> Result<TokenStream, CompileError> {
    Ok(match statement {
        EventStatementIr::LetBool { name, value } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { let mut #name = #value; }
        }
        EventStatementIr::LetUsize { name, value } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { let mut #name = #value; }
        }
        EventStatementIr::LetUsizeStack { name } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { let mut #name: Vec<usize> = Vec::new(); }
        }
        EventStatementIr::SetBool { name, value } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            let value = compile_predicate(value)?;
            quote! { #name = #value; }
        }
        EventStatementIr::SetUsize { name, value } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            let value = compile_usize(value)?;
            quote! { #name = #value; }
        }
        EventStatementIr::CloseThroughLevel { stack, level } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            let level = compile_usize(level)?;
            quote! {
                while #stack.last().is_some_and(|&open| open >= #level) {
                    let _ = #stack.pop();
                    events.push(TreeEvent::FinishNode);
                }
            }
        }
        EventStatementIr::OpenLevel {
            stack,
            level,
            syntax_kind,
        } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            let level = compile_usize(level)?;
            quote! {
                events.push(TreeEvent::StartNode(#syntax_kind));
                #stack.push(#level);
            }
        }
        EventStatementIr::CloseAllLevels { stack } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            quote! {
                while #stack.pop().is_some() {
                    events.push(TreeEvent::FinishNode);
                }
            }
        }
        EventStatementIr::StartNode { syntax_kind } => {
            quote! { events.push(TreeEvent::StartNode(#syntax_kind)); }
        }
        EventStatementIr::Token {
            syntax_kind,
            start,
            end,
        } => compile_token_statement(*syntax_kind, start, end)?,
        EventStatementIr::FinishNode => quote! { events.push(TreeEvent::FinishNode); },
        EventStatementIr::If {
            condition,
            consequent,
            alternate,
        } => compile_if_statement(condition, consequent, alternate)?,
        EventStatementIr::ForLineBytes {
            index,
            from,
            until,
            body,
        } => compile_line_byte_loop(index, from, until, body)?,
        EventStatementIr::WithSourceBounds { from, until, body } => {
            compile_source_bounds(from, until, body)?
        }
        EventStatementIr::CallSourceHelper { name, from, until } => {
            let name = helper_ident(name)?;
            let from = compile_offset(from)?;
            let until = compile_offset(until)?;
            quote! { #name(source, &mut events, #from, #until); }
        }
        EventStatementIr::ScanListMarker { marker } => compile_scan_list_marker(marker)?,
        EventStatementIr::PushFrame { .. } | EventStatementIr::PopFrame { .. } => {
            compile_stack_statement(statement)?
        }
        EventStatementIr::CloseFramesWhile {
            stack,
            condition,
            finish_count,
        } => compile_close_frames_while(stack, condition, *finish_count)?,
        EventStatementIr::CloseAllFrames {
            stack,
            finish_count,
        } => compile_close_all_frames(stack, *finish_count)?,
    })
}

fn compile_stack_statement(statement: &EventStatementIr) -> Result<TokenStream, CompileError> {
    match statement {
        EventStatementIr::PushFrame { stack, value } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            let value = compile_usize(value)?;
            Ok(quote! { #stack.push(#value); })
        }
        EventStatementIr::PopFrame { stack } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            Ok(quote! { let _ = #stack.pop(); })
        }
        _ => Err(CompileError::Schema("expected stack statement".into())),
    }
}

fn compile_line_byte_loop(
    index: &str,
    from: &EventOffsetIr,
    until: &EventOffsetIr,
    body: &[EventStatementIr],
) -> Result<TokenStream, CompileError> {
    let index = line_index_name(index)?;
    let from = compile_offset(from)?;
    let until = compile_offset(until)?;
    let body = compile_statements(body)?;
    Ok(quote! {
        let iteration_from = #from;
        let iteration_until = #until;
        if iteration_from >= start && iteration_from <= iteration_until
            && iteration_until <= end {
            for #index in iteration_from..iteration_until { #body }
        }
    })
}

fn compile_source_bounds(
    from: &EventOffsetIr,
    until: &EventOffsetIr,
    body: &[EventStatementIr],
) -> Result<TokenStream, CompileError> {
    let from = compile_offset(from)?;
    let until = compile_offset(until)?;
    let cached_markers = compile_marker_cache(body)?;
    let body = compile_statements(body)?;
    Ok(quote! {
        let bounds_from = #from;
        let bounds_until = #until;
        if let Some(line) = source.get(bounds_from..bounds_until) {
            let start = bounds_from;
            let end = bounds_until;
            #(#cached_markers)*
            #body
        }
    })
}

fn helper_ident(name: &str) -> Result<syn::Ident, CompileError> {
    let name = syn::parse_str::<syn::Ident>(name)?;
    Ok(syn::parse_str(&format!("__event_helper_{name}"))?)
}

fn validate_helper_calls(
    statements: &[EventStatementIr],
    names: &std::collections::BTreeSet<&str>,
    allow_calls: bool,
) -> Result<(), CompileError> {
    for statement in statements {
        match statement {
            EventStatementIr::CallSourceHelper { name, .. }
                if !allow_calls || !names.contains(name.as_str()) =>
            {
                return Err(CompileError::Schema(format!(
                    "unknown or recursive event helper: {name}"
                )));
            }
            EventStatementIr::If {
                consequent,
                alternate,
                ..
            } => {
                validate_helper_calls(consequent, names, allow_calls)?;
                validate_helper_calls(alternate, names, allow_calls)?;
            }
            EventStatementIr::ForLineBytes { body, .. }
            | EventStatementIr::WithSourceBounds { body, .. } => {
                validate_helper_calls(body, names, allow_calls)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn compile_event_helper(helper: &EventHelperIr) -> Result<TokenStream, CompileError> {
    if helper.initial.iter().any(|statement| {
        !matches!(
            statement,
            EventStatementIr::LetBool { .. }
                | EventStatementIr::LetUsize { .. }
                | EventStatementIr::LetUsizeStack { .. }
        )
    }) {
        return Err(CompileError::Schema(
            "event helper initial state must be declarations".into(),
        ));
    }
    if !compile_future_cache_declarations(&[&helper.body])?.is_empty() {
        return Err(CompileError::Schema(
            "event helpers do not admit future-line searches".into(),
        ));
    }
    let name = helper_ident(&helper.name)?;
    let initial = compile_statements(&helper.initial)?;
    let cached_markers = compile_marker_cache(&helper.body)?;
    let body = compile_statements(&helper.body)?;
    Ok(quote! {
        fn #name(source: &str, events: &mut Vec<TreeEvent>,
                 bounds_from: usize, bounds_until: usize) {
            let bytes = source.as_bytes();
            if let Some(line) = source.get(bounds_from..bounds_until) {
                let _ = line;
                let start = bounds_from;
                let end = bounds_until;
                #initial
                #(#cached_markers)*
                #body
            }
        }
    })
}

fn compile_if_statement(
    condition: &EventPredicateIr,
    consequent: &[EventStatementIr],
    alternate: &[EventStatementIr],
) -> Result<TokenStream, CompileError> {
    let condition = compile_predicate(condition)?;
    let consequent = compile_statements(consequent)?;
    let alternate = compile_statements(alternate)?;
    Ok(if alternate.is_empty() {
        quote! {
            let __event_condition = #condition;
            if __event_condition { #consequent }
        }
    } else {
        quote! {
            let __event_condition = #condition;
            if __event_condition { #consequent } else { #alternate }
        }
    })
}

fn compile_token_statement(
    syntax_kind: u16,
    start: &EventOffsetIr,
    end: &EventOffsetIr,
) -> Result<TokenStream, CompileError> {
    let token_start = compile_offset(start)?;
    let token_end = compile_offset(end)?;
    Ok(quote! {
        let token_start = #token_start;
        let token_end = #token_end;
        if token_start != token_end {
            events.push(TreeEvent::Token {
                kind: #syntax_kind, start: token_start, end: token_end,
            });
        }
    })
}

fn compile_offset(offset: &EventOffsetIr) -> Result<TokenStream, CompileError> {
    Ok(match offset {
        EventOffsetIr::Boundary(EventBoundaryIr::Start) => quote! { start },
        EventOffsetIr::Boundary(EventBoundaryIr::End) => quote! { end },
        EventOffsetIr::Computed(EventComputedOffsetIr::LinePrefixEnd { value }) => {
            let bytes = value.len();
            quote! { start.saturating_add(#bytes).min(end) }
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineSkipHorizontal { from }) => {
            let from = compile_offset(from)?;
            quote! {{
                let mut cursor = #from;
                while cursor < end && matches!(bytes[cursor], b' ' | b'\t') {
                    cursor += 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(
            scan @ (EventComputedOffsetIr::LineScanWord { .. }
            | EventComputedOffsetIr::LineScanKey { .. }
            | EventComputedOffsetIr::LineScanNonspaceUntil { .. }
            | EventComputedOffsetIr::LineScanUntil { .. }),
        ) => compile_scan_offset(scan)?,
        EventOffsetIr::Computed(EventComputedOffsetIr::LineStep { from }) => {
            let from = compile_offset(from)?;
            quote! { (#from).saturating_add(1).min(end) }
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LinePhysicalEnd { from }) => {
            let from = compile_offset(from)?;
            quote! {{
                let mut cursor = (#from).max(start).min(end);
                while cursor < end && !matches!(bytes[cursor], b'\r' | b'\n') {
                    cursor += 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineTrimEnd) => {
            quote! {{
                let mut cursor = end;
                while cursor > start && bytes[cursor - 1].is_ascii_whitespace() {
                    cursor -= 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineTrimEndFrom { from }) => {
            let from = compile_offset(from)?;
            quote! {{
                let floor = #from;
                let mut cursor = end;
                while cursor > floor && bytes[cursor - 1].is_ascii_whitespace() {
                    cursor -= 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineContentEnd) => {
            quote! {{
                let mut cursor = end;
                while cursor > start && matches!(bytes[cursor - 1], b'\r' | b'\n') {
                    cursor -= 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineIndex { name }) => {
            let name = line_index_name(name)?;
            quote! { #name }
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::StateOffset { name }) => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { #name }
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineMarkerEnd { marker, separator }) => {
            let name = marker_name(*marker, *separator)?;
            quote! { start + #name }
        }
    })
}

fn compile_scan_offset(scan: &EventComputedOffsetIr) -> Result<TokenStream, CompileError> {
    let (from, condition) = match scan {
        EventComputedOffsetIr::LineScanWord { from } => (
            from,
            quote! { !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') },
        ),
        EventComputedOffsetIr::LineScanKey { from } => (
            from,
            quote! { bytes[cursor].is_ascii_alphanumeric() || matches!(bytes[cursor], b'_' | b'-') },
        ),
        EventComputedOffsetIr::LineScanNonspaceUntil { from, delimiter } => (
            from,
            quote! { bytes[cursor] != #delimiter && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') },
        ),
        EventComputedOffsetIr::LineScanUntil { from, delimiter } => {
            (from, quote! { bytes[cursor] != #delimiter })
        }
        _ => unreachable!("compile_scan_offset is only called for scan offsets"),
    };
    let from = compile_offset(from)?;
    Ok(quote! {{
        let mut cursor = #from;
        while cursor < end && (#condition) {
            cursor += 1;
        }
        cursor
    }})
}

fn compile_predicate(predicate: &EventPredicateIr) -> Result<TokenStream, CompileError> {
    Ok(match predicate {
        EventPredicateIr::Bool { value } => quote! { #value },
        EventPredicateIr::State { name } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { #name }
        }
        EventPredicateIr::UsizePositive { value } => {
            let value = compile_usize(value)?;
            quote! { (#value) > 0 }
        }
        EventPredicateIr::UsizeEqual { left, right } => {
            let left = compile_usize(left)?;
            let right = compile_usize(right)?;
            quote! { (#left) == (#right) }
        }
        EventPredicateIr::UsizeNotEqual { left, right } => {
            let left = compile_usize(left)?;
            let right = compile_usize(right)?;
            quote! { (#left) != (#right) }
        }
        EventPredicateIr::UsizeGreater { left, right } => {
            let left = compile_usize(left)?;
            let right = compile_usize(right)?;
            quote! { (#left) > (#right) }
        }
        EventPredicateIr::OffsetLess { left, right } => {
            let left = compile_offset(left)?;
            let right = compile_offset(right)?;
            quote! { (#left) < (#right) }
        }
        EventPredicateIr::StackNonempty { stack } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            quote! { !#stack.is_empty() }
        }
        EventPredicateIr::LineStartsWith { .. }
        | EventPredicateIr::LineStartsWithAsciiCaseInsensitive { .. }
        | EventPredicateIr::LinePrefixBoundaryAsciiCaseInsensitive { .. }
        | EventPredicateIr::LineMarkerAsciiCaseInsensitive { .. }
        | EventPredicateIr::LineBlank
        | EventPredicateIr::LineHasWordAfterPrefix { .. }
        | EventPredicateIr::LineHasKeyAfterPrefix { .. } => compile_line_predicate(predicate),
        EventPredicateIr::FutureLineMarkerBeforeBoundary { .. } => {
            compile_future_predicate(predicate)?
        }
        EventPredicateIr::FutureNamedLineMarkerBeforeBoundary { .. } => {
            compile_future_named_marker(predicate)?
        }
        EventPredicateIr::LineByteEqual { at, value } => compile_line_byte_equal(at, *value)?,
        EventPredicateIr::LineBytesAllIn {
            from,
            until,
            values,
        } => compile_line_byte_set(from, until, values, true)?,
        EventPredicateIr::LineBytesAnyIn {
            from,
            until,
            values,
        } => compile_line_byte_set(from, until, values, false)?,
        EventPredicateIr::LineBytesInSet {
            from,
            until,
            values,
        } => compile_line_bytes_in_set(from, until, values)?,
        EventPredicateIr::SourceSlicesEqual {
            left_from,
            left_until,
            right_from,
            right_until,
            ascii_case_insensitive,
        } => compile_source_slices_equal(
            left_from,
            left_until,
            right_from,
            right_until,
            *ascii_case_insensitive,
        )?,
        EventPredicateIr::Not { value } => {
            let value = compile_predicate(value)?;
            quote! { !(#value) }
        }
        EventPredicateIr::And { left, right } => {
            let left = compile_predicate(left)?;
            let right = compile_predicate(right)?;
            quote! { (#left) && (#right) }
        }
        EventPredicateIr::Or { left, right } => {
            let left = compile_predicate(left)?;
            let right = compile_predicate(right)?;
            quote! { (#left) || (#right) }
        }
    })
}

fn compile_line_predicate(predicate: &EventPredicateIr) -> TokenStream {
    match predicate {
        EventPredicateIr::LineStartsWith { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! { line.starts_with(#value) }
        }
        EventPredicateIr::LineStartsWithAsciiCaseInsensitive { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(#value))
            }
        }
        EventPredicateIr::LinePrefixBoundaryAsciiCaseInsensitive { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .filter(|prefix| prefix.eq_ignore_ascii_case(#value))
                    .is_some_and(|_| line.as_bytes().get(#value.len())
                        .is_none_or(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')))
            }
        }
        EventPredicateIr::LineMarkerAsciiCaseInsensitive { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .filter(|prefix| prefix.eq_ignore_ascii_case(#value))
                    .is_some_and(|_| line.as_bytes()[#value.len()..]
                        .iter().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')))
            }
        }
        EventPredicateIr::LineBlank => {
            quote! { line.as_bytes().iter().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')) }
        }
        EventPredicateIr::LineHasWordAfterPrefix { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .filter(|prefix| prefix.eq_ignore_ascii_case(#value))
                    .and_then(|_| line.as_bytes()[#value.len()..]
                        .iter().find(|byte| !matches!(byte, b' ' | b'\t')))
                    .is_some_and(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            }
        }
        EventPredicateIr::LineHasKeyAfterPrefix { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .filter(|prefix| prefix.eq_ignore_ascii_case(#value))
                    .is_some_and(|_| {
                        let mut cursor = #value.len();
                        while cursor < line.len() && (bytes[start + cursor].is_ascii_alphanumeric()
                            || matches!(bytes[start + cursor], b'_' | b'-')) {
                            cursor += 1;
                        }
                        cursor > #value.len() && bytes.get(start + cursor) == Some(&b':')
                    })
            }
        }
        _ => unreachable!("compile_line_predicate only receives line predicates"),
    }
}

fn compile_line_byte_equal(at: &EventOffsetIr, value: u8) -> Result<TokenStream, CompileError> {
    let at = compile_offset(at)?;
    Ok(quote! {{
        let at = #at;
        at >= start && at < end && bytes.get(at) == Some(&#value)
    }})
}

fn compile_line_byte_set(
    from: &EventOffsetIr,
    until: &EventOffsetIr,
    values: &[u8],
    all: bool,
) -> Result<TokenStream, CompileError> {
    let from = compile_offset(from)?;
    let until = compile_offset(until)?;
    let match_slice = if all {
        quote! { slice.iter().all(|byte| [#(#values),*].contains(byte)) }
    } else {
        quote! { slice.iter().any(|byte| [#(#values),*].contains(byte)) }
    };
    Ok(quote! {{
        let from = #from;
        let until = #until;
        from >= start && from <= until && until <= end
            && bytes.get(from..until).is_some_and(|slice| #match_slice)
    }})
}

fn compile_line_bytes_in_set(
    from: &EventOffsetIr,
    until: &EventOffsetIr,
    values: &[String],
) -> Result<TokenStream, CompileError> {
    if values.is_empty()
        || values
            .iter()
            .any(|value| value.is_empty() || !value.is_ascii())
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(CompileError::Schema(
            "line byte name set must be nonempty, ASCII, sorted, and unique".into(),
        ));
    }
    let from = compile_offset(from)?;
    let until = compile_offset(until)?;
    let names = values
        .iter()
        .map(|value| syn::LitByteStr::new(value.as_bytes(), proc_macro2::Span::call_site()));
    Ok(quote! {{
        let from = #from;
        let until = #until;
        const NAMES: &[&[u8]] = &[#(#names),*];
        from >= start && from <= until && until <= end
            && bytes.get(from..until).is_some_and(|slice| NAMES.binary_search(&slice).is_ok())
    }})
}

fn compile_source_slices_equal(
    left_from: &EventOffsetIr,
    left_until: &EventOffsetIr,
    right_from: &EventOffsetIr,
    right_until: &EventOffsetIr,
    ascii_case_insensitive: bool,
) -> Result<TokenStream, CompileError> {
    let left_from = compile_offset(left_from)?;
    let left_until = compile_offset(left_until)?;
    let right_from = compile_offset(right_from)?;
    let right_until = compile_offset(right_until)?;
    let compare = if ascii_case_insensitive {
        quote! { left.eq_ignore_ascii_case(right) }
    } else {
        quote! { left == right }
    };
    Ok(quote! {{
        let left_from = #left_from;
        let left_until = #left_until;
        let right_from = #right_from;
        let right_until = #right_until;
        left_from <= left_until && right_from <= right_until
            && bytes.get(left_from..left_until).zip(bytes.get(right_from..right_until))
                .is_some_and(|(left, right)| #compare)
    }})
}

fn compile_usize(value: &EventUsizeIr) -> Result<TokenStream, CompileError> {
    Ok(match value {
        EventUsizeIr::Usize { value } => quote! { #value },
        EventUsizeIr::State { name } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { #name }
        }
        EventUsizeIr::LineMarkerLevel { marker, separator } => {
            let name = marker_name(*marker, *separator)?;
            quote! { #name }
        }
        EventUsizeIr::Offset { value } => compile_offset(value)?,
        EventUsizeIr::LineIndentColumn { tab_width } => {
            if *tab_width == 0 {
                return Err(CompileError::Schema("tab width must be positive".into()));
            }
            quote! {{
                let mut cursor = start;
                let mut column = 0usize;
                while cursor < end && matches!(bytes[cursor], b' ' | b'\t') {
                    if bytes[cursor] == b'\t' {
                        column = (column / #tab_width + 1) * #tab_width;
                    } else {
                        column += 1;
                    }
                    cursor += 1;
                }
                column
            }}
        }
        EventUsizeIr::StackTop { stack } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            quote! { #stack.last().copied().unwrap_or(0) }
        }
        EventUsizeIr::Add { left, right } => {
            let left = compile_usize(left)?;
            let right = compile_usize(right)?;
            quote! { (#left).saturating_add(#right) }
        }
        EventUsizeIr::Multiply { left, right } => {
            let left = compile_usize(left)?;
            let right = compile_usize(right)?;
            quote! { (#left).saturating_mul(#right) }
        }
        EventUsizeIr::Divide { left, right } => {
            let left = compile_usize(left)?;
            match right.as_ref() {
                EventUsizeIr::Usize { value } if *value > 0 => quote! { (#left) / #value },
                _ => {
                    let right = compile_usize(right)?;
                    quote! { (#left) / (#right).max(1) }
                }
            }
        }
    })
}
