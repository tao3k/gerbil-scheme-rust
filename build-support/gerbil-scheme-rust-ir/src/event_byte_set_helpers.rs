//! Shared pure byte-set predicates for Scheme-owned event transitions.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::visit::Visit;

pub(super) fn compile_byte_set_helpers(body: &TokenStream) -> Result<TokenStream, syn::Error> {
    let block = syn::parse2::<syn::Block>(quote! {{ #body }})?;
    let mut calls = ByteSetCalls::default();
    calls.visit_block(&block);
    let any = calls.0.contains(&ByteSetKind::Any).then(|| {
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
    let all = calls.0.contains(&ByteSetKind::All).then(|| {
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
    let mask_any = calls.0.contains(&ByteSetKind::MaskAny).then(|| {
        quote! {
            #[inline]
            fn __event_any_byte_in_mask(
                bytes: &[u8], start: usize, end: usize,
                from: usize, until: usize, mask: [u64; 4],
            ) -> bool {
                from >= start && from <= until && until <= end
                    && bytes.get(from..until).is_some_and(|slice| {
                        slice.iter().any(|&byte| {
                            mask[usize::from(byte / 64)] & (1u64 << (byte % 64)) != 0
                        })
                    })
            }
        }
    });
    let mask_all = calls.0.contains(&ByteSetKind::MaskAll).then(|| {
        quote! {
            #[inline]
            fn __event_all_bytes_in_mask(
                bytes: &[u8], start: usize, end: usize,
                from: usize, until: usize, mask: [u64; 4],
            ) -> bool {
                from >= start && from <= until && until <= end
                    && bytes.get(from..until).is_some_and(|slice| {
                        slice.iter().all(|&byte| {
                            mask[usize::from(byte / 64)] & (1u64 << (byte % 64)) != 0
                        })
                    })
            }
        }
    });
    Ok(quote! { #any #all #mask_any #mask_all })
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ByteSetKind {
    Any,
    All,
    MaskAny,
    MaskAll,
}

#[derive(Default)]
struct ByteSetCalls(BTreeSet<ByteSetKind>);

impl<'ast> Visit<'ast> for ByteSetCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            let kind = if path.path.is_ident("__event_any_byte_in") {
                Some(ByteSetKind::Any)
            } else if path.path.is_ident("__event_all_bytes_in") {
                Some(ByteSetKind::All)
            } else if path.path.is_ident("__event_any_byte_in_mask") {
                Some(ByteSetKind::MaskAny)
            } else if path.path.is_ident("__event_all_bytes_in_mask") {
                Some(ByteSetKind::MaskAll)
            } else {
                None
            };
            if let Some(kind) = kind {
                self.0.insert(kind);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}
