use gerbil_scheme::{SchemeBorrowedBytevector, SchemeBorrowedVector, SchemeScalar};

#[test]
fn scheme_scalar_projects_to_public_sys_abi_shapes() {
    let fixnum = SchemeScalar::from(-7_isize);
    assert_eq!(fixnum.as_fixnum_abi().map(|value| value.0), Some(-7));
    assert!(fixnum.as_boolean_abi().is_none());

    let boolean = SchemeScalar::from(true);
    assert_eq!(boolean.as_boolean_abi().map(|value| value.0), Some(1));
    assert!(boolean.as_char_abi().is_none());

    let character = SchemeScalar::from('λ');
    assert_eq!(
        character.as_char_abi().map(|value| value.0),
        Some('λ' as u32)
    );
    assert!(character.as_flonum_abi().is_none());

    let flonum = SchemeScalar::from(1.5_f64);
    assert_eq!(flonum.as_flonum_abi().map(|value| value.0), Some(1.5));
    assert!(flonum.as_fixnum_abi().is_none());
}

#[test]
fn scheme_borrowed_bytevector_preserves_slice_and_abi_projection() {
    let bytes = [1_u8, 2, 3, 5, 8];
    let borrowed = SchemeBorrowedBytevector::new(&bytes);
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_bytes(), bytes);
    assert_eq!(abi.ptr, bytes.as_ptr());
    assert_eq!(abi.len, bytes.len());
}

#[test]
fn scheme_borrowed_vector_preserves_handle_slice_and_abi_projection() {
    let mut left = 1_u8;
    let mut right = 2_u8;
    let handles = [
        (&raw mut left).cast::<core::ffi::c_void>(),
        (&raw mut right).cast::<core::ffi::c_void>(),
    ];
    let borrowed = SchemeBorrowedVector::new(&handles);
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_values(), handles);
    assert_eq!(abi.ptr, handles.as_ptr());
    assert_eq!(abi.len, handles.len());
}
