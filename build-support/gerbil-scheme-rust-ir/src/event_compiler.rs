//! Bounded stateful event procedures lowered from Scheme-owned parser IR.
//!
//! This backend owns no language rule: it validates a closed event vocabulary
//! and constructs Rust syntax. The Scheme frontend owns every predicate and
//! transition supplied through the versioned IR.

use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::collections::BTreeSet;

use crate::CompileError;

#[path = "event_list_compiler.rs"]
mod event_list_compiler;
use event_list_compiler::{
    compile_close_all_frames, compile_close_frames_while, compile_scan_list_marker,
};

/// Versioned wire contract for source-backed event functions.
pub const EVENT_FUNCTION_IR_SCHEMA: &str = "gerbil-scheme-rust.event-function-ir.v1";

/// One Scheme-authored event procedure with line-local and final transitions.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFunctionIr {
    /// Exact supported wire schema.
    pub schema: String,
    /// Public generated Rust function name.
    pub name: String,
    /// Declared root syntax-kind index.
    pub root_kind: u16,
    /// Digest of the source-owned parser algorithm.
    pub parser_digest: String,
    /// Initial state declarations, evaluated before the line fold.
    pub initial: Vec<EventStatementIr>,
    /// One transition for each source line.
    pub line: Vec<EventStatementIr>,
    /// Final transitions before the root closes.
    pub finish: Vec<EventStatementIr>,
}

/// Closed, auditable side effects permitted in an event procedure.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventStatementIr {
    /// Declare one bounded mutable boolean state slot.
    LetBool { name: String, value: bool },
    /// Declare one unsigned parser state slot.
    LetUsize { name: String, value: usize },
    /// Declare a nesting stack of unsigned levels.
    LetUsizeStack { name: String },
    /// Update a previously declared state slot.
    SetBool {
        name: String,
        value: EventPredicateIr,
    },
    /// Update a previously declared unsigned state slot.
    SetUsize { name: String, value: EventUsizeIr },
    /// Close all nested nodes at or above a new level.
    CloseThroughLevel { stack: String, level: EventUsizeIr },
    /// Open one nested node at a declared level.
    OpenLevel {
        stack: String,
        level: EventUsizeIr,
        syntax_kind: u16,
    },
    /// Close every remaining node in a nesting stack.
    CloseAllLevels { stack: String },
    /// Emit an opening Rowan node event.
    StartNode { syntax_kind: u16 },
    /// Emit a source-backed token for the current line.
    Token {
        syntax_kind: u16,
        start: EventOffsetIr,
        end: EventOffsetIr,
    },
    /// Emit a closing Rowan node event.
    FinishNode,
    /// Choose one statically bounded transition branch.
    If {
        condition: EventPredicateIr,
        consequent: Vec<Self>,
        alternate: Vec<Self>,
    },
    /// Iterate a bounded source-line byte range; the index is source-backed.
    ForLineBytes {
        index: String,
        from: EventOffsetIr,
        until: EventOffsetIr,
        body: Vec<Self>,
    },
    /// Read a source-line list marker into typed state slots.
    ScanListMarker { marker: EventListMarkerIr },
    /// Push an unsigned frame owned by the Scheme transition.
    PushFrame { stack: String, value: EventUsizeIr },
    /// Pop frames while a Scheme predicate holds, emitting a fixed close arity.
    CloseFramesWhile {
        stack: String,
        condition: EventPredicateIr,
        finish_count: u8,
    },
    /// Close every frame, emitting a fixed number of node closes per frame.
    CloseAllFrames { stack: String, finish_count: u8 },
}

/// Language-declared list marker shape and typed event-state destinations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventListMarkerIr {
    /// Unordered one-byte marker alternatives.
    pub unordered: String,
    /// Whether decimal and alphabetic ordered bullets are admitted.
    pub ordered: bool,
    /// Positive tab stop width used for indentation columns.
    pub tab_width: usize,
    /// Boolean slot for a successfully recognized marker.
    pub present: String,
    /// Unsigned indentation column slot.
    pub column: String,
    /// Boolean slot for ordered/unordered shape.
    pub ordered_slot: String,
    /// Source-backed bullet-start slot.
    pub bullet_start: String,
    /// Source-backed bullet-end slot.
    pub bullet_end: String,
    /// Source-backed content-start slot.
    pub content_start: String,
}

/// Source byte offsets available in a line transition.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum EventOffsetIr {
    /// An existing current-line boundary.
    Boundary(EventBoundaryIr),
    /// A statically bounded source-line byte calculation.
    Computed(EventComputedOffsetIr),
}

/// Current source-line boundaries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventBoundaryIr {
    /// Inclusive current-line start.
    Start,
    /// Exclusive current-line end.
    End,
}

/// Bounded source-line byte offsets without arbitrary Rust expressions.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventComputedOffsetIr {
    /// End of a declared literal prefix, clamped to the current line.
    LinePrefixEnd { value: String },
    /// Skip spaces and tabs from a source-backed offset.
    LineSkipHorizontal { from: Box<EventOffsetIr> },
    /// Scan the next whitespace-delimited word from a source-backed offset.
    LineScanWord { from: Box<EventOffsetIr> },
    /// Scan an ASCII identifier composed of letters, digits, underscore and hyphen.
    LineScanKey { from: Box<EventOffsetIr> },
    /// Move one byte forward without passing the current line boundary.
    LineStep { from: Box<EventOffsetIr> },
    /// Remove trailing ASCII whitespace from the current source line.
    LineTrimEnd,
    /// Trim trailing whitespace without crossing a source-backed value start.
    LineTrimEndFrom { from: Box<EventOffsetIr> },
    /// End before a final CR/LF, preserving horizontal source whitespace.
    LineContentEnd,
    /// A byte index bound by a surrounding source-line iteration.
    LineIndex { name: String },
    /// A named unsigned state containing a current source-line offset.
    StateOffset { name: String },
    /// End of a checked leading marker run; reuses the line's cached level.
    LineMarkerEnd { marker: u8, separator: u8 },
}

/// Closed unsigned-value vocabulary for contextual line transitions.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventUsizeIr {
    /// A literal nonnegative value.
    Usize { value: usize },
    /// A named unsigned parser state slot.
    State { name: String },
    /// Count repeated leading marker bytes only when followed by a separator.
    LineMarkerLevel { marker: u8, separator: u8 },
    /// Promote a checked source-backed offset to an unsigned state value.
    Offset { value: EventOffsetIr },
    /// Tab-aware leading indentation width of the current source line.
    LineIndentColumn { tab_width: usize },
    /// Top of a nonempty unsigned frame stack; zero when empty.
    StackTop { stack: String },
    /// Checked unsigned arithmetic for encoded frame values.
    Add { left: Box<Self>, right: Box<Self> },
    /// Checked multiplication for encoded frame values.
    Multiply { left: Box<Self>, right: Box<Self> },
    /// Bounded positive-divisor quotient for encoded frame values.
    Divide { left: Box<Self>, right: Box<Self> },
}

/// Closed predicate vocabulary; it has no arbitrary Rust expression node.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPredicateIr {
    /// A literal truth value.
    Bool { value: bool },
    /// Read one named state slot.
    State { name: String },
    /// Test whether an unsigned expression is positive.
    UsizePositive { value: EventUsizeIr },
    /// Compare two typed unsigned parser values.
    UsizeEqual {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Distinguish two typed unsigned parser values.
    UsizeNotEqual {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Compare two unsigned parser values.
    UsizeGreater {
        left: EventUsizeIr,
        right: EventUsizeIr,
    },
    /// Compare two source-backed byte offsets.
    OffsetLess {
        left: EventOffsetIr,
        right: EventOffsetIr,
    },
    /// Test whether a named unsigned frame stack contains a frame.
    StackNonempty { stack: String },
    /// Compare the current source line's prefix.
    LineStartsWith { value: String },
    /// Compare a source-line prefix using ASCII-insensitive syntax matching.
    LineStartsWithAsciiCaseInsensitive { value: String },
    /// Match an ASCII-insensitive prefix only at a horizontal or line boundary.
    LinePrefixBoundaryAsciiCaseInsensitive { value: String },
    /// Match a whole line marker with only trailing ASCII whitespace.
    LineMarkerAsciiCaseInsensitive { value: String },
    /// Treat spaces, tabs, and line endings as a blank source line.
    LineBlank,
    /// A declared prefix is followed by one nonempty whitespace-delimited word.
    LineHasWordAfterPrefix { value: String },
    /// A prefix is followed by a nonempty ASCII key and a colon.
    LineHasKeyAfterPrefix { value: String },
    /// Compare one source-line byte at a bounded source-backed offset.
    LineByteEqual { at: EventOffsetIr, value: u8 },
    /// All bytes in one bounded slice belong to an explicit byte set.
    LineBytesAllIn {
        from: EventOffsetIr,
        until: EventOffsetIr,
        values: Vec<u8>,
    },
    /// At least one byte in one bounded slice belongs to an explicit byte set.
    LineBytesAnyIn {
        from: EventOffsetIr,
        until: EventOffsetIr,
        values: Vec<u8>,
    },
    /// Boolean negation.
    Not { value: Box<Self> },
    /// Short-circuit conjunction.
    And { left: Box<Self>, right: Box<Self> },
    /// Short-circuit disjunction.
    Or { left: Box<Self>, right: Box<Self> },
}

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
    let name = syn::parse_str::<syn::Ident>(&function.name)?;
    let root = function.root_kind;
    let digest = syn::LitStr::new(&function.parser_digest, proc_macro2::Span::call_site());
    let initial = compile_statements(&function.initial)?;
    let line = compile_statements(&function.line)?;
    let finish = compile_statements(&function.finish)?;
    let cached_markers = compile_marker_cache(&function.line)?;
    let tokens = quote! {
        pub const PARSER_DIGEST: &str = #digest;

        pub fn #name(source: &str) -> Vec<TreeEvent> {
            let bytes = source.as_bytes();
            let mut events = Vec::with_capacity(bytes.len() / 16 + 2);
            events.push(TreeEvent::StartNode(#root));
            #initial
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

fn collect_line_markers(statements: &[EventStatementIr], markers: &mut BTreeSet<(u8, u8)>) {
    for statement in statements {
        match statement {
            EventStatementIr::SetUsize { value, .. }
            | EventStatementIr::CloseThroughLevel { level: value, .. }
            | EventStatementIr::OpenLevel { level: value, .. }
            | EventStatementIr::PushFrame { value, .. } => {
                collect_usize_markers(value, markers);
            }
            EventStatementIr::SetBool { value, .. } => collect_predicate_markers(value, markers),
            EventStatementIr::Token { start, end, .. } => {
                collect_offset_markers(start, markers);
                collect_offset_markers(end, markers);
            }
            EventStatementIr::If {
                condition,
                consequent,
                alternate,
            } => {
                collect_predicate_markers(condition, markers);
                collect_line_markers(consequent, markers);
                collect_line_markers(alternate, markers);
            }
            EventStatementIr::ForLineBytes {
                from, until, body, ..
            } => {
                collect_offset_markers(from, markers);
                collect_offset_markers(until, markers);
                collect_line_markers(body, markers);
            }
            EventStatementIr::CloseFramesWhile { condition, .. } => {
                collect_predicate_markers(condition, markers);
            }
            _ => {}
        }
    }
}

fn collect_offset_markers(offset: &EventOffsetIr, markers: &mut BTreeSet<(u8, u8)>) {
    match offset {
        EventOffsetIr::Computed(EventComputedOffsetIr::LineMarkerEnd { marker, separator }) => {
            markers.insert((*marker, *separator));
        }
        EventOffsetIr::Computed(
            EventComputedOffsetIr::LineSkipHorizontal { from }
            | EventComputedOffsetIr::LineScanWord { from }
            | EventComputedOffsetIr::LineScanKey { from }
            | EventComputedOffsetIr::LineStep { from }
            | EventComputedOffsetIr::LineTrimEndFrom { from },
        ) => collect_offset_markers(from, markers),
        _ => {}
    }
}

fn collect_predicate_markers(predicate: &EventPredicateIr, markers: &mut BTreeSet<(u8, u8)>) {
    match predicate {
        EventPredicateIr::UsizePositive { value } => collect_usize_markers(value, markers),
        EventPredicateIr::UsizeEqual { left, right }
        | EventPredicateIr::UsizeNotEqual { left, right }
        | EventPredicateIr::UsizeGreater { left, right } => {
            collect_usize_markers(left, markers);
            collect_usize_markers(right, markers);
        }
        EventPredicateIr::OffsetLess { left, right } => {
            collect_offset_markers(left, markers);
            collect_offset_markers(right, markers);
        }
        EventPredicateIr::LineByteEqual { at, .. } => collect_offset_markers(at, markers),
        EventPredicateIr::LineBytesAllIn { from, until, .. }
        | EventPredicateIr::LineBytesAnyIn { from, until, .. } => {
            collect_offset_markers(from, markers);
            collect_offset_markers(until, markers);
        }
        EventPredicateIr::Not { value } => collect_predicate_markers(value, markers),
        EventPredicateIr::And { left, right } | EventPredicateIr::Or { left, right } => {
            collect_predicate_markers(left, markers);
            collect_predicate_markers(right, markers);
        }
        _ => {}
    }
}

fn collect_usize_markers(value: &EventUsizeIr, markers: &mut BTreeSet<(u8, u8)>) {
    match value {
        EventUsizeIr::LineMarkerLevel { marker, separator } => {
            markers.insert((*marker, *separator));
        }
        EventUsizeIr::Offset { value } => collect_offset_markers(value, markers),
        EventUsizeIr::Add { left, right }
        | EventUsizeIr::Multiply { left, right }
        | EventUsizeIr::Divide { left, right } => {
            collect_usize_markers(left, markers);
            collect_usize_markers(right, markers);
        }
        _ => {}
    }
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
        EventStatementIr::ScanListMarker { marker } => compile_scan_list_marker(marker)?,
        EventStatementIr::PushFrame { stack, value } => {
            let stack = syn::parse_str::<syn::Ident>(stack)?;
            let value = compile_usize(value)?;
            quote! { #stack.push(#value); }
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
        EventOffsetIr::Computed(EventComputedOffsetIr::LineScanWord { from }) => {
            let from = compile_offset(from)?;
            quote! {{
                let mut cursor = #from;
                while cursor < end && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
                    cursor += 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineScanKey { from }) => {
            let from = compile_offset(from)?;
            quote! {{
                let mut cursor = #from;
                while cursor < end && (bytes[cursor].is_ascii_alphanumeric()
                    || matches!(bytes[cursor], b'_' | b'-')) {
                    cursor += 1;
                }
                cursor
            }}
        }
        EventOffsetIr::Computed(EventComputedOffsetIr::LineStep { from }) => {
            let from = compile_offset(from)?;
            quote! { (#from).saturating_add(1).min(end) }
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
