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
    /// Compare the current source line's prefix.
    LineStartsWith { value: String },
    /// Compare a source-line prefix using ASCII-insensitive syntax matching.
    LineStartsWithAsciiCaseInsensitive { value: String },
    /// Treat spaces, tabs, and line endings as a blank source line.
    LineBlank,
    /// A declared prefix is followed by one nonempty whitespace-delimited word.
    LineHasWordAfterPrefix { value: String },
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
            | EventStatementIr::OpenLevel { level: value, .. } => {
                collect_usize_markers(value, markers);
            }
            EventStatementIr::SetBool { value, .. } => collect_predicate_markers(value, markers),
            EventStatementIr::If {
                condition,
                consequent,
                alternate,
            } => {
                collect_predicate_markers(condition, markers);
                collect_line_markers(consequent, markers);
                collect_line_markers(alternate, markers);
            }
            _ => {}
        }
    }
}

fn collect_predicate_markers(predicate: &EventPredicateIr, markers: &mut BTreeSet<(u8, u8)>) {
    match predicate {
        EventPredicateIr::UsizePositive { value } => collect_usize_markers(value, markers),
        EventPredicateIr::Not { value } => collect_predicate_markers(value, markers),
        EventPredicateIr::And { left, right } | EventPredicateIr::Or { left, right } => {
            collect_predicate_markers(left, markers);
            collect_predicate_markers(right, markers);
        }
        _ => {}
    }
}

fn collect_usize_markers(value: &EventUsizeIr, markers: &mut BTreeSet<(u8, u8)>) {
    if let EventUsizeIr::LineMarkerLevel { marker, separator } = value {
        markers.insert((*marker, *separator));
    }
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
        } => {
            let token_start = compile_offset(start)?;
            let token_end = compile_offset(end)?;
            quote! {
                events.push(TreeEvent::Token {
                    kind: #syntax_kind, start: #token_start, end: #token_end
                });
            }
        }
        EventStatementIr::FinishNode => quote! { events.push(TreeEvent::FinishNode); },
        EventStatementIr::If {
            condition,
            consequent,
            alternate,
        } => {
            let condition = compile_predicate(condition)?;
            let consequent = compile_statements(consequent)?;
            let alternate = compile_statements(alternate)?;
            if alternate.is_empty() {
                quote! { if #condition { #consequent } }
            } else {
                quote! { if #condition { #consequent } else { #alternate } }
            }
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
        EventPredicateIr::LineBlank => {
            quote! { line.as_bytes().iter().all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n')) }
        }
        EventPredicateIr::LineHasWordAfterPrefix { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! {
                line.get(..#value.len())
                    .filter(|prefix| prefix.eq_ignore_ascii_case(#value))
                    .and_then(|_| line.as_bytes()[#value.len()..]
                        .iter().skip_while(|byte| matches!(byte, b' ' | b'\t')).next())
                    .is_some_and(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            }
        }
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
    })
}
