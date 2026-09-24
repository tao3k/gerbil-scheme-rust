//! Generic source-line list marker and bounded frame lowering.

use proc_macro2::TokenStream;
use quote::quote;

use super::{CompileError, EventListMarkerIr, EventPredicateIr, compile_predicate};

pub(super) fn compile_scan_list_marker(
    marker: &EventListMarkerIr,
) -> Result<TokenStream, CompileError> {
    if marker.tab_width == 0 || marker.unordered.is_empty() || !marker.unordered.is_ascii() {
        return Err(CompileError::Schema(
            "invalid list marker declaration".into(),
        ));
    }
    let unordered = marker.unordered.as_bytes();
    let ordered_bullet = if marker.ordered {
        quote! {
            if first.is_ascii_digit() {
                let mut cursor = __event_list_cursor + 1;
                while cursor < __event_list_content_end && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                if cursor < __event_list_content_end && matches!(bytes[cursor], b'.' | b')') {
                    Some((true, cursor + 1))
                } else {
                    None
                }
            } else if first.is_ascii_alphabetic()
                && __event_list_cursor + 1 < __event_list_content_end
                && matches!(bytes[__event_list_cursor + 1], b'.' | b')') {
                Some((true, __event_list_cursor + 2))
            } else {
                None
            }
        }
    } else {
        quote! { None }
    };
    let tab_width = marker.tab_width;
    let present = syn::parse_str::<syn::Ident>(&marker.present)?;
    let column = syn::parse_str::<syn::Ident>(&marker.column)?;
    let ordered_slot = syn::parse_str::<syn::Ident>(&marker.ordered_slot)?;
    let bullet_start = syn::parse_str::<syn::Ident>(&marker.bullet_start)?;
    let bullet_end = syn::parse_str::<syn::Ident>(&marker.bullet_end)?;
    let content_start = syn::parse_str::<syn::Ident>(&marker.content_start)?;
    Ok(quote! {
        let __event_list_content_end = {
            let mut cursor = end;
            while cursor > start && matches!(bytes[cursor - 1], b'\r' | b'\n') {
                cursor -= 1;
            }
            cursor
        };
        let mut __event_list_cursor = start;
        let mut __event_list_column = 0usize;
        while __event_list_cursor < __event_list_content_end
            && matches!(bytes[__event_list_cursor], b' ' | b'\t') {
            if bytes[__event_list_cursor] == b'\t' {
                __event_list_column = (__event_list_column / #tab_width + 1) * #tab_width;
            } else {
                __event_list_column += 1;
            }
            __event_list_cursor += 1;
        }
        let __event_list_marker = if __event_list_cursor < __event_list_content_end {
            let first = bytes[__event_list_cursor];
            let bullet = if [#(#unordered),*].contains(&first) {
                Some((false, __event_list_cursor + 1))
            } else {
                #ordered_bullet
            };
            bullet.and_then(|(is_ordered, bullet_limit)| {
                if bullet_limit < __event_list_content_end
                    && !matches!(bytes[bullet_limit], b' ' | b'\t') {
                    return None;
                }
                let mut content = bullet_limit;
                while content < __event_list_content_end
                    && matches!(bytes[content], b' ' | b'\t') {
                    content += 1;
                }
                Some((__event_list_column, is_ordered, __event_list_cursor,
                      bullet_limit, content))
            })
        } else {
            None
        };
        // A scan is a state transition; acknowledge the previous slot value
        // before replacing it so declared initial state remains warning-free.
        let _ = #present;
        if let Some((marker_column, marker_ordered, marker_start, marker_end, marker_content))
            = __event_list_marker {
            #present = true;
            #column = marker_column;
            #ordered_slot = marker_ordered;
            #bullet_start = marker_start;
            #bullet_end = marker_end;
            #content_start = marker_content;
        } else {
            #present = false;
        }
    })
}

pub(super) fn compile_close_frames_while(
    stack: &str,
    condition: &EventPredicateIr,
    finish_count: u8,
) -> Result<TokenStream, CompileError> {
    if finish_count == 0 {
        return Err(CompileError::Schema(
            "frame close arity must be positive".into(),
        ));
    }
    let stack = syn::parse_str::<syn::Ident>(stack)?;
    let condition = compile_predicate(condition)?;
    Ok(quote! {
        while #stack.last().is_some() {
            let __event_close_frame = #condition;
            if !__event_close_frame { break; }
            let _ = #stack.pop();
            for _ in 0..#finish_count { events.push(TreeEvent::FinishNode); }
        }
    })
}

pub(super) fn compile_close_all_frames(
    stack: &str,
    finish_count: u8,
) -> Result<TokenStream, CompileError> {
    if finish_count == 0 {
        return Err(CompileError::Schema(
            "frame close arity must be positive".into(),
        ));
    }
    let stack = syn::parse_str::<syn::Ident>(stack)?;
    Ok(quote! {
        while #stack.pop().is_some() {
            for _ in 0..#finish_count { events.push(TreeEvent::FinishNode); }
        }
    })
}
