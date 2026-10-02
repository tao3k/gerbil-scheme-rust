//! Source-backed Rowan token emission from a typed event transition.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventOffsetIr, compile_offset};

pub(super) fn compile_token_statement(
    syntax_kind: u16,
    start: &EventOffsetIr,
    end: &EventOffsetIr,
    in_helper: bool,
) -> Result<TokenStream, CompileError> {
    let token_start = compile_offset(start)?;
    let token_end = compile_offset(end)?;
    let events = if in_helper {
        quote! { &mut *events }
    } else {
        quote! { &mut events }
    };
    Ok(quote! {
        __event_push_token(#events, #syntax_kind, #token_start, #token_end);
    })
}
