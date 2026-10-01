//! Shared pure source-offset operations for Scheme-owned event transitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;

pub(super) fn compile_offset_helpers(body: &TokenStream) -> Result<TokenStream, syn::Error> {
    let block = syn::parse2::<syn::Block>(quote! {{ #body }})?;
    let mut calls = OffsetCalls::default();
    calls.visit_block(&block);
    let mut helpers = TokenStream::new();
    compile_line_helpers(calls, &mut helpers);
    compile_scan_helpers(calls, &mut helpers);
    Ok(helpers)
}

fn compile_line_helpers(calls: OffsetCalls, helpers: &mut TokenStream) {
    if calls.has(OffsetCalls::SKIP_HORIZONTAL) {
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
    if calls.has(OffsetCalls::TRIM_WHITESPACE_END) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_trim_whitespace_end(bytes: &[u8], floor: usize, end: usize) -> usize {
                let mut cursor = end;
                while cursor > floor && bytes[cursor - 1].is_ascii_whitespace() {
                    cursor -= 1;
                }
                cursor
            }
        });
    }
    if calls.has(OffsetCalls::LINE_CONTENT_END) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_line_content_end(bytes: &[u8], start: usize, end: usize) -> usize {
                let mut cursor = end;
                while cursor > start && matches!(bytes[cursor - 1], b'\r' | b'\n') {
                    cursor -= 1;
                }
                cursor
            }
        });
    }
    if calls.has(OffsetCalls::LINE_PHYSICAL_END) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_line_physical_end(
                bytes: &[u8], start: usize, end: usize, from: usize,
            ) -> usize {
                let mut cursor = from.max(start).min(end);
                while cursor < end && !matches!(bytes[cursor], b'\r' | b'\n') {
                    cursor += 1;
                }
                cursor
            }
        });
    }
}

fn compile_scan_helpers(calls: OffsetCalls, helpers: &mut TokenStream) {
    if calls.has(OffsetCalls::SCAN_WORD) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_scan_word(bytes: &[u8], end: usize, from: usize) -> usize {
                let mut cursor = from;
                while cursor < end && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
                    cursor += 1;
                }
                cursor
            }
        });
    }
    if calls.has(OffsetCalls::SCAN_KEY) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_scan_key(bytes: &[u8], end: usize, from: usize) -> usize {
                let mut cursor = from;
                while cursor < end
                    && (bytes[cursor].is_ascii_alphanumeric()
                        || matches!(bytes[cursor], b'_' | b'-'))
                {
                    cursor += 1;
                }
                cursor
            }
        });
    }
    if calls.has(OffsetCalls::SCAN_NONSPACE_UNTIL) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_scan_nonspace_until(
                bytes: &[u8], end: usize, from: usize, delimiter: u8,
            ) -> usize {
                let mut cursor = from;
                while cursor < end
                    && bytes[cursor] != delimiter
                    && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n')
                {
                    cursor += 1;
                }
                cursor
            }
        });
    }
    if calls.has(OffsetCalls::SCAN_UNTIL) {
        helpers.extend(quote! {
            #[inline(always)]
            fn __event_scan_until(
                bytes: &[u8], end: usize, from: usize, delimiter: u8,
            ) -> usize {
                let mut cursor = from;
                while cursor < end && bytes[cursor] != delimiter {
                    cursor += 1;
                }
                cursor
            }
        });
    }
}

#[derive(Clone, Copy, Default)]
struct OffsetCalls(u8);

impl OffsetCalls {
    const SKIP_HORIZONTAL: u8 = 1 << 0;
    const TRIM_WHITESPACE_END: u8 = 1 << 1;
    const LINE_CONTENT_END: u8 = 1 << 2;
    const LINE_PHYSICAL_END: u8 = 1 << 3;
    const SCAN_WORD: u8 = 1 << 4;
    const SCAN_KEY: u8 = 1 << 5;
    const SCAN_NONSPACE_UNTIL: u8 = 1 << 6;
    const SCAN_UNTIL: u8 = 1 << 7;

    fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

impl<'ast> Visit<'ast> for OffsetCalls {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            for (name, flag) in [
                ("__event_skip_horizontal", Self::SKIP_HORIZONTAL),
                ("__event_trim_whitespace_end", Self::TRIM_WHITESPACE_END),
                ("__event_line_content_end", Self::LINE_CONTENT_END),
                ("__event_line_physical_end", Self::LINE_PHYSICAL_END),
                ("__event_scan_word", Self::SCAN_WORD),
                ("__event_scan_key", Self::SCAN_KEY),
                ("__event_scan_nonspace_until", Self::SCAN_NONSPACE_UNTIL),
                ("__event_scan_until", Self::SCAN_UNTIL),
            ] {
                if path.path.is_ident(name) {
                    self.0 |= flag;
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}
