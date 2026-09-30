//! Shared pure source-offset operations for Scheme-owned event transitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;

pub(super) fn compile_offset_helpers(body: &TokenStream) -> Result<TokenStream, syn::Error> {
    let block = syn::parse2::<syn::Block>(quote! {{ #body }})?;
    let mut calls = OffsetCalls::default();
    calls.visit_block(&block);
    let mut helpers = TokenStream::new();
    if calls.skip_horizontal {
        helpers.extend(quote! {
            #[inline]
            fn __event_skip_horizontal(bytes: &[u8], end: usize, from: usize) -> usize {
                let mut cursor = from;
                while cursor < end && matches!(bytes[cursor], b' ' | b'\t') {
                    cursor += 1;
                }
                cursor
            }
        });
    }
    if calls.trim_whitespace_end {
        helpers.extend(quote! {
            #[inline]
            fn __event_trim_whitespace_end(bytes: &[u8], floor: usize, end: usize) -> usize {
                let mut cursor = end;
                while cursor > floor && bytes[cursor - 1].is_ascii_whitespace() {
                    cursor -= 1;
                }
                cursor
            }
        });
    }
    if calls.line_content_end {
        helpers.extend(quote! {
            #[inline]
            fn __event_line_content_end(bytes: &[u8], start: usize, end: usize) -> usize {
                let mut cursor = end;
                while cursor > start && matches!(bytes[cursor - 1], b'\r' | b'\n') {
                    cursor -= 1;
                }
                cursor
            }
        });
    }
    Ok(helpers)
}

#[derive(Default)]
struct OffsetCalls {
    skip_horizontal: bool,
    trim_whitespace_end: bool,
    line_content_end: bool,
}

impl<'ast> Visit<'ast> for OffsetCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            self.skip_horizontal |= path.path.is_ident("__event_skip_horizontal");
            self.trim_whitespace_end |= path.path.is_ident("__event_trim_whitespace_end");
            self.line_content_end |= path.path.is_ident("__event_line_content_end");
        }
        syn::visit::visit_expr_call(self, call);
    }
}
