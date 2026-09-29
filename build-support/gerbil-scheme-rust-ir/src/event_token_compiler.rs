//! Source-backed Rowan token emission from a typed event transition.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventOffsetIr, compile_offset};

pub(super) fn compile_token_statement(
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
