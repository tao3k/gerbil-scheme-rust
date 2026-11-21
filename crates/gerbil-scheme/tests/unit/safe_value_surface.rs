use gerbil_scheme::{
    GerbilStatus, GerbilValue, GerbilValueProvenance, SchemeBorrowedBytevector,
    SchemeBorrowedVector, SchemeKeyword, SchemeList, SchemePair, SchemeScalar, SchemeSymbol,
};

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
    let handles = [(&raw mut left).addr(), (&raw mut right).addr()];
    let borrowed = SchemeBorrowedVector::new(&handles);
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_values(), handles);
    assert_eq!(abi.ptr, handles.as_ptr());
    assert_eq!(abi.len, handles.len());
}

#[test]
fn scheme_handle_backed_views_preserve_identity_and_reject_null() {
    let mut raw_value = 42_u8;
    let raw = (&raw mut raw_value).addr();

    let symbol = SchemeSymbol::from_raw(raw).expect("non-zero symbol handle");
    let keyword = SchemeKeyword::from_raw(raw).expect("non-zero keyword handle");
    let pair = SchemePair::from_raw(raw).expect("non-zero pair handle");
    let list = SchemeList::from_raw(raw).expect("non-zero list handle");

    assert_eq!(symbol.as_raw(), raw);
    assert_eq!(keyword.as_raw(), raw);
    assert_eq!(pair.as_raw(), raw);
    assert_eq!(list.as_raw(), raw);

    assert!(SchemeSymbol::from_raw(0).is_none());
    assert!(SchemeKeyword::from_raw(0).is_none());
    assert!(SchemePair::from_raw(0).is_none());
    assert!(SchemeList::from_raw(0).is_none());
}

#[test]
fn scheme_pair_list_status_wrappers_fail_closed_for_unbacked_handles() {
    let mut raw_value = 42_u8;
    let raw = (&raw mut raw_value).addr();
    let value = || GerbilValue::from_raw(raw).expect("non-zero value handle");
    assert_eq!(value().provenance(), GerbilValueProvenance::UntrustedRaw);

    let is_pair = value().is_pair();
    assert!(is_pair.is_err());
    assert_eq!(is_pair.status(), Some(GerbilStatus::InvalidValue));

    let is_list = value().is_list();
    assert!(is_list.is_err());
    assert_eq!(is_list.status(), Some(GerbilStatus::InvalidValue));

    let is_null = value().is_null();
    assert!(is_null.is_err());
    assert_eq!(is_null.status(), Some(GerbilStatus::InvalidValue));

    let car = value().pair_car();
    assert!(car.is_err());
    assert_eq!(car.status(), Some(GerbilStatus::InvalidValue));

    let cdr = value().pair_cdr();
    assert!(cdr.is_err());
    assert_eq!(cdr.status(), Some(GerbilStatus::InvalidValue));

    let parts = value().pair_parts();
    assert!(parts.is_err());
    assert_eq!(parts.status(), Some(GerbilStatus::InvalidValue));

    assert!(GerbilValue::from_raw(0).is_err());
}
