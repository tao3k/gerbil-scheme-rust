//! Shared pure source-offset operations for Scheme-owned event transitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;

pub(super) fn compile_offset_helpers(body: &TokenStream) -> Result<TokenStream, syn::Error> {
    let block = syn::parse2::<syn::Block>(quote! {{ #body }})?;
    let mut calls = OffsetCalls::default();
    calls.visit_block(&block);
    Ok(if calls.skip_horizontal {
        quote! {
            #[inline]
            fn __event_skip_horizontal(bytes: &[u8], end: usize, from: usize) -> usize {
                let mut cursor = from;
                while cursor < end && matches!(bytes[cursor], b' ' | b'\t') {
                    cursor += 1;
                }
                cursor
            }
        }
    } else {
        TokenStream::new()
    })
}

#[derive(Default)]
struct OffsetCalls {
    skip_horizontal: bool,
}

impl<'ast> Visit<'ast> for OffsetCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            self.skip_horizontal |= path.path.is_ident("__event_skip_horizontal");
        }
        syn::visit::visit_expr_call(self, call);
    }
}
