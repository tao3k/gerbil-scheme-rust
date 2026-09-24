//! Build-time lowering of typed Scheme AOT function IR into Rust tokens.
//!
//! This crate does not embed Gerbil or own a language parser. A Scheme compiler
//! supplies an admitted IR value; this backend constructs Rust tokens and asks
//! `syn` to validate the resulting function before it is written or compiled.
//! Source-level purity and call admission remain the Scheme compiler's job.

use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::{error::Error, fmt};

/// Versioned wire shape shared by Scheme compilers and build-time Rust callers.
pub const FUNCTION_IR_SCHEMA: &str = "gerbil-scheme-rust.rust-function-ir.v1";

/// A source-owned pure function, with Rust types made explicit at the boundary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionIr {
    /// The exact wire contract, rejected when stale or unsupported.
    pub schema: String,
    /// Public Rust function name.
    pub name: String,
    /// Typed function parameters.
    pub parameters: Vec<ParameterIr>,
    /// Rust result type.
    pub result: String,
    /// Pure function body.
    pub body: BlockIr,
}

/// A named parameter whose Rust type is parsed before code emission.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterIr {
    /// Parameter name.
    pub name: String,
    /// Rust type spelling, such as `&str` or `&[String]`.
    pub ty: String,
}

/// Ordered immutable bindings followed by one result expression.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockIr {
    /// Local bindings evaluated in declaration order.
    pub bindings: Vec<BindingIr>,
    /// Final expression.
    pub result: ExprIr,
}

/// One immutable local binding.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingIr {
    /// Local binding name.
    pub name: String,
    /// Value expression.
    pub value: ExprIr,
}

/// The closed expression shape admitted by the AOT backend; it has no raw-code node.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExprIr {
    /// A Rust string literal.
    String { value: String },
    /// A bound name or admitted Rust path.
    Name { value: String },
    /// A typed function or constructor call.
    Call {
        callee: Box<Self>,
        arguments: Vec<Self>,
    },
    /// A method call on an expression.
    Method {
        receiver: Box<Self>,
        method: String,
        arguments: Vec<Self>,
    },
    /// Positional projection of a tuple value.
    TupleIndex { tuple: Box<Self>, index: usize },
    /// Prefix before the first delimiter occurrence.
    Before {
        value: Box<Self>,
        delimiter: Box<Self>,
        owned: bool,
    },
    /// Suffix after the first delimiter occurrence.
    After {
        value: Box<Self>,
        delimiter: Box<Self>,
    },
    /// First whitespace-separated word.
    FirstWord { value: Box<Self> },
    /// Whitespace-separated word iterator.
    Words { value: Box<Self> },
    /// Pure existential traversal.
    Any {
        collection: Box<Self>,
        variable: String,
        body: Box<Self>,
        string_slice: bool,
    },
    /// Empty collection or string predicate.
    Empty { value: Box<Self> },
    /// String membership in a slice.
    StringIn {
        value: Box<Self>,
        collection: Box<Self>,
    },
    /// Pure conditional.
    If {
        condition: Box<Self>,
        consequent: Box<Self>,
        alternate: Box<Self>,
    },
    /// Bounded boolean or equality operator.
    Binary {
        operator: BinaryOperator,
        left: Box<Self>,
        right: Box<Self>,
    },
}

/// Operators admitted by pure Scheme lowering.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOperator {
    /// Logical disjunction.
    Or,
    /// Logical conjunction.
    And,
    /// Value equality.
    Equal,
}

/// A malformed wire value or invalid Rust syntax produced from it.
#[derive(Debug)]
pub enum CompileError {
    /// The JSON wire value is invalid.
    Json(serde_json::Error),
    /// The wire version is unsupported.
    Schema(String),
    /// An identifier, path, type, or emitted function is invalid Rust.
    Syntax(syn::Error),
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid Scheme AOT IR: {error}"),
            Self::Schema(schema) => write!(formatter, "unsupported Scheme AOT IR schema: {schema}"),
            Self::Syntax(error) => write!(formatter, "invalid generated Rust syntax: {error}"),
        }
    }
}

impl Error for CompileError {}

impl From<syn::Error> for CompileError {
    fn from(error: syn::Error) -> Self {
        Self::Syntax(error)
    }
}

/// Parse a versioned Scheme AOT IR document and compile it into Rust source.
///
/// # Errors
/// Rejects malformed JSON, unknown wire revisions, and invalid Rust syntax.
pub fn compile_function_json(input: &str) -> Result<String, CompileError> {
    let function: FunctionIr = serde_json::from_str(input).map_err(CompileError::Json)?;
    compile_function(&function)
}

/// Construct Rust tokens from typed IR and validate the complete function.
///
/// # Errors
/// Rejects unknown wire revisions and invalid Rust identifiers, paths, types,
/// or expressions.
pub fn compile_function(function: &FunctionIr) -> Result<String, CompileError> {
    if function.schema != FUNCTION_IR_SCHEMA {
        return Err(CompileError::Schema(function.schema.clone()));
    }
    let name = syn::parse_str::<syn::Ident>(&function.name)?;
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| {
            let name = syn::parse_str::<syn::Ident>(&parameter.name)?;
            let ty = syn::parse_str::<syn::Type>(&parameter.ty)?;
            Ok(quote! { #name: #ty })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    let result = syn::parse_str::<syn::Type>(&function.result)?;
    let bindings = function
        .body
        .bindings
        .iter()
        .map(|binding| {
            let name = syn::parse_str::<syn::Ident>(&binding.name)?;
            let value = compile_expression(&binding.value)?;
            Ok(quote! { let #name = #value; })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    let tail = compile_expression(&function.body.result)?;
    let tokens = quote! {
        pub fn #name(#(#parameters),*) -> #result {
            #(#bindings)*
            #tail
        }
    };
    syn::parse2::<syn::ItemFn>(tokens.clone())?;
    Ok(tokens.to_string())
}

fn compile_expression(expression: &ExprIr) -> Result<TokenStream, CompileError> {
    Ok(match expression {
        ExprIr::String { value } => {
            let literal = syn::LitStr::new(value, proc_macro2::Span::call_site());
            quote! { #literal }
        }
        ExprIr::Name { value } => {
            let path = syn::parse_str::<syn::Path>(value)?;
            quote! { #path }
        }
        ExprIr::Call { callee, arguments } => {
            let callee = compile_expression(callee)?;
            let arguments = compile_arguments(arguments)?;
            quote! { #callee(#(#arguments),*) }
        }
        ExprIr::Method {
            receiver,
            method,
            arguments,
        } => {
            let receiver = compile_expression(receiver)?;
            let method = syn::parse_str::<syn::Ident>(method)?;
            let arguments = compile_arguments(arguments)?;
            quote! { #receiver.#method(#(#arguments),*) }
        }
        ExprIr::TupleIndex { tuple, index } => {
            let tuple = compile_expression(tuple)?;
            let index = syn::Index::from(*index);
            quote! { #tuple.#index }
        }
        ExprIr::Before {
            value,
            delimiter,
            owned,
        } => compile_before(value, delimiter, *owned)?,
        ExprIr::After { value, delimiter } => {
            let value = compile_expression(value)?;
            let delimiter = compile_expression(delimiter)?;
            quote! { #value.split_once(#delimiter).map_or("", |(_, tail)| tail) }
        }
        ExprIr::FirstWord { value } => {
            let value = compile_expression(value)?;
            quote! { #value.split_whitespace().next().unwrap_or("") }
        }
        ExprIr::Words { value } => {
            let value = compile_expression(value)?;
            quote! { #value.split_whitespace() }
        }
        ExprIr::Any {
            collection,
            variable,
            body,
            string_slice,
        } => compile_any(collection, variable, body, *string_slice)?,
        ExprIr::Empty { value } => {
            let value = compile_expression(value)?;
            quote! { #value.is_empty() }
        }
        ExprIr::StringIn { value, collection } => {
            let value = compile_expression(value)?;
            let collection = compile_expression(collection)?;
            quote! { #collection.contains(&#value) }
        }
        ExprIr::If {
            condition,
            consequent,
            alternate,
        } => {
            let condition = compile_expression(condition)?;
            let consequent = compile_expression(consequent)?;
            let alternate = compile_expression(alternate)?;
            quote! { if #condition { #consequent } else { #alternate } }
        }
        ExprIr::Binary {
            operator,
            left,
            right,
        } => {
            let left = compile_expression(left)?;
            let right = compile_expression(right)?;
            match operator {
                BinaryOperator::Or => quote! { (#left) || (#right) },
                BinaryOperator::And => quote! { (#left) && (#right) },
                BinaryOperator::Equal => quote! { (#left) == (#right) },
            }
        }
    })
}

fn compile_arguments(arguments: &[ExprIr]) -> Result<Vec<TokenStream>, CompileError> {
    arguments.iter().map(compile_expression).collect()
}

fn compile_before(
    value: &ExprIr,
    delimiter: &ExprIr,
    owned: bool,
) -> Result<TokenStream, CompileError> {
    let value = compile_expression(value)?;
    let delimiter = compile_expression(delimiter)?;
    Ok(if owned {
        quote! { #value.split_once(#delimiter).map_or(#value, |(head, _)| head).to_owned() }
    } else {
        quote! { #value.split_once(#delimiter).map_or(#value, |(head, _)| head) }
    })
}

fn compile_any(
    collection: &ExprIr,
    variable: &str,
    body: &ExprIr,
    string_slice: bool,
) -> Result<TokenStream, CompileError> {
    let collection = compile_expression(collection)?;
    let variable = syn::parse_str::<syn::Ident>(variable)?;
    let body = compile_expression(body)?;
    Ok(if string_slice {
        quote! { #collection.iter().map(String::as_str).any(|#variable| #body) }
    } else {
        quote! { #collection.any(|#variable| #body) }
    })
}
