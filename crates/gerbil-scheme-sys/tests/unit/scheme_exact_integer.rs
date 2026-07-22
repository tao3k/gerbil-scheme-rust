use crate::{
    GerbilBoolean, GerbilRootId, GerbilStatus,
    gerbil_scheme_rust_fixture_exact_integer_large_negative,
    gerbil_scheme_rust_fixture_exact_integer_large_positive,
    gerbil_scheme_rust_i64_to_exact_integer_root, gerbil_scheme_rust_root_exact_integer_to_i64,
    gerbil_scheme_rust_root_exact_integer_to_u64,
    gerbil_scheme_rust_scheme_object_exact_integer_to_i64,
    gerbil_scheme_rust_scheme_object_exact_integer_to_u64,
    gerbil_scheme_rust_scheme_object_is_exact_integer,
    gerbil_scheme_rust_u64_to_exact_integer_root,
};

#[test]
fn exact_integer_checked_abi_rejects_null_boundaries_before_raw_ffi() {
    let mut boolean = GerbilBoolean::FALSE;
    let mut signed = 0_i64;
    let mut unsigned = 0_u64;

    unsafe {
        assert_eq!(
            gerbil_scheme_rust_fixture_exact_integer_large_positive(core::ptr::null_mut()),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_fixture_exact_integer_large_negative(core::ptr::null_mut()),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_scheme_object_is_exact_integer(0, &raw mut boolean),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_scheme_object_exact_integer_to_i64(0, &raw mut signed),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_scheme_object_exact_integer_to_u64(0, &raw mut unsigned),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_i64_to_exact_integer_root(0, core::ptr::null_mut()),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_u64_to_exact_integer_root(0, core::ptr::null_mut()),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_root_exact_integer_to_i64(GerbilRootId(0), &raw mut signed),
            GerbilStatus::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_root_exact_integer_to_u64(GerbilRootId(0), &raw mut unsigned),
            GerbilStatus::NullPointer
        );
    }
}
