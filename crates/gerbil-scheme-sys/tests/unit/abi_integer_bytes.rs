use crate::{
    GerbilByteOrder, GerbilRootId, GerbilStatus, gerbil_scheme_rust_bytevector_to_uint,
    gerbil_scheme_rust_root_bytevector_to_uint, gerbil_scheme_rust_uint_to_bytevector_root,
};

#[test]
fn byte_order_codes_are_stable() {
    assert_eq!(GerbilByteOrder::Big.code(), 0);
    assert_eq!(GerbilByteOrder::Little.code(), 1);
    assert_eq!(GerbilByteOrder::Native.code(), 2);
}

#[test]
fn checked_surface_rejects_invalid_inputs_before_raw_ffi() {
    let mut uint = 7_u64;
    let mut root = GerbilRootId(7);

    // SAFETY: invalid inputs intentionally stop before the raw Gerbil call.
    assert_eq!(
        unsafe { gerbil_scheme_rust_bytevector_to_uint(0, 0, 1, &raw mut uint) },
        GerbilStatus::NullPointer
    );
    // SAFETY: invalid inputs intentionally stop before the raw Gerbil call.
    assert_eq!(
        unsafe { gerbil_scheme_rust_root_bytevector_to_uint(GerbilRootId(0), 0, 1, &raw mut uint) },
        GerbilStatus::NullPointer
    );
    // SAFETY: null output intentionally stops before the raw Gerbil call.
    assert_eq!(
        unsafe { gerbil_scheme_rust_uint_to_bytevector_root(1, 0, 1, core::ptr::null_mut()) },
        GerbilStatus::NullPointer
    );
    // SAFETY: invalid order intentionally stops before the raw Gerbil call.
    assert_eq!(
        unsafe { gerbil_scheme_rust_uint_to_bytevector_root(1, 99, 1, &raw mut root) },
        GerbilStatus::InvalidValue
    );
    // SAFETY: invalid width intentionally stops before the raw Gerbil call.
    assert_eq!(
        unsafe { gerbil_scheme_rust_uint_to_bytevector_root(1, 0, 9, &raw mut root) },
        GerbilStatus::InvalidValue
    );
    assert_eq!(uint, 7);
    assert_eq!(root, GerbilRootId(7));
}
