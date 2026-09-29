//! Typed runtime parameters for Scheme-authored event fold state.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::TokenStream;
use quote::quote;

use super::{
    EventFunctionIr, EventHelperIr, EventOffsetIr, EventStatementIr, EventUsizeIr,
    compile_statement,
};
use crate::CompileError;

pub(super) fn compile_event_parameters(
    function: &EventFunctionIr,
    name: &syn::Ident,
) -> Result<(syn::Ident, Vec<TokenStream>, TokenStream), CompileError> {
    if function.parameters.is_empty() {
        return Ok((name.clone(), Vec::new(), quote! {}));
    }
    let declared = function
        .initial
        .iter()
        .filter_map(|statement| match statement {
            EventStatementIr::LetUsize { name, value } => Some((name.as_str(), *value)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let state_names = function
        .initial
        .iter()
        .filter_map(|statement| match statement {
            EventStatementIr::LetBool { name, .. }
            | EventStatementIr::LetUsize { name, .. }
            | EventStatementIr::LetUsizeStack { name } => Some(name.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut names = BTreeSet::new();
    let mut states = BTreeSet::new();
    let mut parameters = Vec::new();
    let mut defaults = Vec::new();
    for parameter in &function.parameters {
        if !names.insert(parameter.name.as_str())
            || !states.insert(parameter.state.as_str())
            || declared.get(parameter.state.as_str()) != Some(&parameter.default)
            || state_names.contains(parameter.name.as_str())
            || matches!(
                parameter.name.as_str(),
                "source" | "bytes" | "events" | "start" | "end" | "line"
            )
        {
            return Err(CompileError::Schema(
                "invalid event parameter declaration".into(),
            ));
        }
        let argument = syn::parse_str::<syn::Ident>(&parameter.name)?;
        let default = parameter.default;
        parameters.push(quote! { #argument: usize });
        defaults.push(quote! { #default });
    }
    let configured = syn::parse_str::<syn::Ident>(&format!("{}_with_parameters", function.name))?;
    let wrapper = quote! {
        pub fn #name(source: &str) -> Vec<TreeEvent> {
            #configured(source, #(#defaults),*)
        }
    };
    Ok((configured, parameters, wrapper))
}

pub(super) fn compile_event_initial(
    function: &EventFunctionIr,
) -> Result<TokenStream, CompileError> {
    let arguments = function
        .parameters
        .iter()
        .map(|parameter| (parameter.state.as_str(), parameter.name.as_str()))
        .collect::<BTreeMap<_, _>>();
    let tokens = function
        .initial
        .iter()
        .map(|statement| match statement {
            EventStatementIr::LetUsize { name, .. } if arguments.contains_key(name.as_str()) => {
                let state = syn::parse_str::<syn::Ident>(name)?;
                let argument = syn::parse_str::<syn::Ident>(arguments[name.as_str()])?;
                if parameter_is_mutated(&function.line, name)
                    || parameter_is_mutated(&function.finish, name)
                {
                    Ok(quote! { let mut #state = #argument; })
                } else {
                    Ok(quote! { let #state = #argument; })
                }
            }
            _ => compile_statement(statement, false),
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    Ok(quote! { #(#tokens)* })
}

fn parameter_is_mutated(statements: &[EventStatementIr], target: &str) -> bool {
    statements.iter().any(|statement| match statement {
        EventStatementIr::SetUsize { name, .. } => name == target,
        EventStatementIr::ScanListMarker { marker } => {
            marker.column == target
                || marker.bullet_start == target
                || marker.bullet_end == target
                || marker.content_start == target
        }
        EventStatementIr::If {
            consequent,
            alternate,
            ..
        } => parameter_is_mutated(consequent, target) || parameter_is_mutated(alternate, target),
        EventStatementIr::JoinOnce {
            branches, fallback, ..
        } => parameter_is_mutated(branches, target) || parameter_is_mutated(fallback, target),
        EventStatementIr::ForLineBytes { body, .. }
        | EventStatementIr::WithSourceBounds { body, .. } => parameter_is_mutated(body, target),
        _ => false,
    })
}

pub(super) fn compile_helper_parameters(
    helper: &EventHelperIr,
) -> Result<(Vec<TokenStream>, TokenStream), CompileError> {
    let declared = helper
        .initial
        .iter()
        .filter_map(|statement| match statement {
            EventStatementIr::LetUsize { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let parameters = helper
        .parameters
        .iter()
        .map(|state| {
            let name = state.as_str();
            if !declared.contains(name) || !seen.insert(name) {
                return Err(CompileError::Schema(
                    "invalid event helper parameter state".into(),
                ));
            }
            let argument = helper_argument_ident(name)?;
            Ok(quote! { #argument: usize })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    let initial = helper
        .initial
        .iter()
        .map(|statement| match statement {
            EventStatementIr::LetUsize { name, .. } if seen.contains(name.as_str()) => {
                let state = syn::parse_str::<syn::Ident>(name)?;
                let argument = helper_argument_ident(name)?;
                if parameter_is_mutated(&helper.body, name) {
                    Ok(quote! { let mut #state = #argument; })
                } else {
                    Ok(quote! { let #state = #argument; })
                }
            }
            EventStatementIr::LetUsize { name, .. }
                if !seen.contains(name.as_str())
                    && matches!(helper.body.first(),
                        Some(EventStatementIr::SetUsize {
                            name: target,
                            value: EventUsizeIr::Offset {
                                value: EventOffsetIr::Boundary(_),
                            },
                        }) if target == name) =>
            {
                let state = syn::parse_str::<syn::Ident>(name)?;
                Ok(quote! { let mut #state: usize; })
            }
            _ => compile_statement(statement, true),
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    Ok((parameters, quote! { #(#initial)* }))
}

fn helper_argument_ident(name: &str) -> Result<syn::Ident, CompileError> {
    let state = syn::parse_str::<syn::Ident>(name)?;
    Ok(syn::parse_str(&format!("__event_arg_{state}"))?)
}
