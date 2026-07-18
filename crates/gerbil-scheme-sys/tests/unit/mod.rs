use core::ffi::c_char;

use crate::{
    GERBIL_SCHEME_RUST_ABI_ID, GERBIL_SCHEME_RUST_ABI_VERSION, GerbilBorrowedUtf8, GerbilStatus,
};

#[test]
fn abi_identity_is_nul_terminated() {
    assert_eq!(GERBIL_SCHEME_RUST_ABI_ID.last(), Some(&0));
    assert_eq!(GERBIL_SCHEME_RUST_ABI_VERSION, 1);
}

#[test]
fn public_header_matches_the_live_scalar_abi() {
    let header = crate::GERBIL_SCHEME_RUST_HEADER_SOURCE;

    assert!(header.contains("#define GERBIL_SCHEME_RUST_ABI_VERSION 1u"));
    assert!(header.contains("GERBIL_STATUS_ABI_MISMATCH = 2"));
    assert!(header.contains("int64_t gerbil_scheme_rust_add_i64(int64_t left, int64_t right);"));
    assert!(header.contains("int32_t gerbil_scheme_rust_is_even_i64(int64_t value);"));
    assert!(!header.contains("int64_t *result"));
}

#[test]
fn status_values_are_stable() {
    assert_eq!(GerbilStatus::Ok as i32, 0);
    assert_eq!(GerbilStatus::Panic as i32, 5);
    assert_eq!(GerbilStatus::AlreadyInitialized as i32, 6);
    assert_eq!(GerbilStatus::NotInitialized as i32, 7);
    assert_eq!(GerbilStatus::RuntimeFinalized as i32, 8);
}

#[test]
fn borrowed_utf8_matches_pointer_and_length_layout() {
    assert_eq!(
        core::mem::size_of::<GerbilBorrowedUtf8>(),
        core::mem::size_of::<*const c_char>() + core::mem::size_of::<usize>()
    );
}
