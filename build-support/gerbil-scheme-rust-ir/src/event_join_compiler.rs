//! Lexically scoped branch joins for Scheme-authored event transitions.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventPredicateIr, EventStatementIr, compile_statements};

fn marks_handled(statement: &EventStatementIr, handled: &str) -> bool {
    match statement {
        EventStatementIr::SetBool {
            name,
            value: EventPredicateIr::Bool { value: true },
        } => name == handled,
        EventStatementIr::If {
            consequent,
            alternate,
            ..
        } => consequent
            .iter()
            .chain(alternate)
            .any(|item| marks_handled(item, handled)),
        EventStatementIr::ForLineBytes { body, .. }
        | EventStatementIr::WithSourceBounds { body, .. } => {
            body.iter().any(|item| marks_handled(item, handled))
        }
        _ => false,
    }
}

pub(super) fn compile_join_once(
    handled: &str,
    branches: &[EventStatementIr],
    continuation: &[EventStatementIr],
    in_helper: bool,
) -> Result<TokenStream, CompileError> {
    if branches.is_empty() || continuation.is_empty() {
        return Err(CompileError::Schema(
            "event join requires branches and a normal continuation".into(),
        ));
    }
    if !branches.iter().any(|item| marks_handled(item, handled)) {
        return Err(CompileError::Schema(
            "event join branches must explicitly mark the handled path".into(),
        ));
    }
    let handled = syn::parse_str::<syn::Ident>(handled)?;
    let branches = compile_statements(branches, in_helper)?;
    let continuation = compile_statements(continuation, in_helper)?;
    Ok(quote! {{
        let mut #handled = false;
        #branches
        if !#handled { #continuation }
    }})
}
