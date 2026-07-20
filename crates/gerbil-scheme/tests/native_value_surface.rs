#![cfg(feature = "native")]

use gerbil_scheme::{GerbilStatus, GerbilUtf8, GerbilValue, NativeError};

#[test]
fn borrowed_utf8_surface_preserves_text_and_abi_bytes() {
    let text = "λ gerbil 🐹";
    let borrowed = GerbilUtf8::new(text);
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_str(), text);
    assert_eq!(borrowed.len(), text.len());
    assert!(!borrowed.is_empty());
    assert!(!abi.ptr.is_null());

    let bytes = unsafe { std::slice::from_raw_parts(abi.ptr.cast::<u8>(), abi.len) };
    assert_eq!(bytes, text.as_bytes());
}

#[test]
fn empty_borrowed_utf8_keeps_zero_length_contract() {
    let borrowed = GerbilUtf8::from("");
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_str(), "");
    assert_eq!(borrowed.len(), 0);
    assert!(borrowed.is_empty());
    assert_eq!(abi.len, 0);
}

#[test]
fn value_handle_rejects_null_without_crossing_ffi() {
    let err = GerbilValue::from_raw(std::ptr::null_mut()).expect_err("null handle must fail");

    assert_eq!(
        err,
        NativeError::Status {
            operation: "GerbilValue::from_raw",
            code: GerbilStatus::NullPointer as i32,
        }
    );
}

#[test]
fn value_handle_preserves_non_null_raw_identity_without_deref() {
    let raw = std::ptr::NonNull::<u8>::dangling()
        .as_ptr()
        .cast::<std::ffi::c_void>();
    let value = GerbilValue::from_raw(raw).expect("non-null opaque handle");

    assert_eq!(value.as_raw(), raw);
}
