//! Typed event state declaration and update lowering.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventStatementIr, compile_predicate, compile_usize};

pub(super) fn compile_state_statement(
    statement: &EventStatementIr,
) -> Result<TokenStream, CompileError> {
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
        _ => unreachable!("compile_state_statement only handles state statements"),
    })
}
