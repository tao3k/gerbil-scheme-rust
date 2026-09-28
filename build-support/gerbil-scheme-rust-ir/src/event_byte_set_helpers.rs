//! Shared pure byte-set predicates for Scheme-owned event transitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;

pub(super) fn compile_byte_set_helpers(body: &TokenStream) -> Result<TokenStream, syn::Error> {
    let block = syn::parse2::<syn::Block>(quote! {{ #body }})?;
    let mut calls = ByteSetCalls::default();
    calls.visit_block(&block);
    let any = calls.any.then(|| {
        quote! {
            #[inline]
            fn __event_any_byte_in(
                bytes: &[u8], start: usize, end: usize,
                from: usize, until: usize, values: &[u8],
            ) -> bool {
                from >= start && from <= until && until <= end
                    && bytes.get(from..until).is_some_and(|slice| {
                        slice.iter().any(|byte| values.contains(byte))
                    })
            }
        }
    });
    let all = calls.all.then(|| {
        quote! {
            #[inline]
            fn __event_all_bytes_in(
                bytes: &[u8], start: usize, end: usize,
                from: usize, until: usize, values: &[u8],
            ) -> bool {
                from >= start && from <= until && until <= end
                    && bytes.get(from..until).is_some_and(|slice| {
                        slice.iter().all(|byte| values.contains(byte))
                    })
            }
        }
    });
    Ok(quote! { #any #all })
}

#[derive(Default)]
struct ByteSetCalls {
    any: bool,
    all: bool,
}

impl<'ast> Visit<'ast> for ByteSetCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            self.any |= path.path.is_ident("__event_any_byte_in");
            self.all |= path.path.is_ident("__event_all_bytes_in");
        }
        syn::visit::visit_expr_call(self, call);
    }
}
