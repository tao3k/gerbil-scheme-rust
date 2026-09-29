//! Lexically scoped branch joins for Scheme-authored event transitions.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventStatementIr, compile_statements};

pub(super) fn compile_join_once(
    handled: &str,
    branches: &[EventStatementIr],
    fallback: &[EventStatementIr],
    in_helper: bool,
) -> Result<TokenStream, CompileError> {
    let handled = syn::parse_str::<syn::Ident>(handled)?;
    let branches = compile_statements(branches, in_helper)?;
    let fallback = compile_statements(fallback, in_helper)?;
    Ok(quote! {{
        let mut #handled = false;
        #branches
        if !#handled { #fallback }
    }})
}
