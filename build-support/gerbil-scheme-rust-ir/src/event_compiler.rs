//! Bounded stateful event procedures lowered from Scheme-owned parser IR.
//!
//! This backend owns no language rule: it validates a closed event vocabulary
//! and constructs Rust syntax. The Scheme frontend owns every predicate and
//! transition supplied through the versioned IR.

use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

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
    /// Update a previously declared state slot.
    SetBool {
        name: String,
        value: EventPredicateIr,
    },
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
#[serde(rename_all = "snake_case")]
pub enum EventOffsetIr {
    /// Inclusive current-line start.
    Start,
    /// Exclusive current-line end.
    End,
}

/// Closed predicate vocabulary; it has no arbitrary Rust expression node.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPredicateIr {
    /// A literal truth value.
    Bool { value: bool },
    /// Read one named state slot.
    State { name: String },
    /// Compare the current source line's prefix.
    LineStartsWith { value: String },
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
                #line
                start = end;
            }
            #finish
            events.push(TreeEvent::FinishNode);
            events
        }
    };
    syn::parse2::<syn::File>(tokens.clone())?;
    Ok(tokens.to_string())
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
        EventStatementIr::SetBool { name, value } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            let value = compile_predicate(value)?;
            quote! { #name = #value; }
        }
        EventStatementIr::StartNode { syntax_kind } => {
            quote! { events.push(TreeEvent::StartNode(#syntax_kind)); }
        }
        EventStatementIr::Token {
            syntax_kind,
            start,
            end,
        } => {
            let start = compile_offset(start);
            let end = compile_offset(end);
            quote! {
                events.push(TreeEvent::Token {
                    kind: #syntax_kind,
                    start: #start,
                    end: #end,
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
            quote! { if #condition { #consequent } else { #alternate } }
        }
    })
}

fn compile_offset(offset: &EventOffsetIr) -> TokenStream {
    match offset {
        EventOffsetIr::Start => quote! { start },
        EventOffsetIr::End => quote! { end },
    }
}

fn compile_predicate(predicate: &EventPredicateIr) -> Result<TokenStream, CompileError> {
    Ok(match predicate {
        EventPredicateIr::Bool { value } => quote! { #value },
        EventPredicateIr::State { name } => {
            let name = syn::parse_str::<syn::Ident>(name)?;
            quote! { #name }
        }
        EventPredicateIr::LineStartsWith { value } => {
            let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! { line.starts_with(#value) }
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
