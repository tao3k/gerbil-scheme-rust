//! Scheme-object flonum fixture and projection ABI wrappers.

use crate::abi::{
    GerbilBoolean, GerbilFlonum, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate,
};

unsafe extern "C" {
    fn gerbil_scheme_rust_fixture_flonum_finite_raw() -> GerbilValueHandle;

    fn gerbil_scheme_rust_fixture_flonum_nan_raw() -> GerbilValueHandle;

    fn gerbil_scheme_rust_fixture_flonum_pos_inf_raw() -> GerbilValueHandle;

    fn gerbil_scheme_rust_fixture_flonum_neg_inf_raw() -> GerbilValueHandle;

    fn gerbil_scheme_rust_fixture_flonum_neg_zero_raw() -> GerbilValueHandle;

    fn gerbil_scheme_rust_scheme_object_is_flonum_raw(value: GerbilValueHandle) -> i32;

    fn gerbil_scheme_rust_scheme_object_flonum_value_raw(value: GerbilValueHandle) -> f64;
}

/// Export a finite Scheme flonum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_flonum_finite(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_flonum_finite_raw) }
}

/// Export a NaN Scheme flonum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_flonum_nan(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_flonum_nan_raw) }
}

/// Export a positive infinity Scheme flonum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_flonum_pos_inf(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_flonum_pos_inf_raw) }
}

/// Export a negative infinity Scheme flonum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_flonum_neg_inf(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_flonum_neg_inf_raw) }
}

/// Export a negative zero Scheme flonum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_flonum_neg_zero(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_flonum_neg_zero_raw) }
}

/// Checks whether a Scheme-object export is a flonum.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_flonum(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    // SAFETY: raw predicate call is guarded by the shared checked predicate helper.
    unsafe {
        checked_scheme_object_predicate(value, out, gerbil_scheme_rust_scheme_object_is_flonum_raw)
    }
}

/// Projects a Scheme-object flonum into the flonum ABI carrier.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilFlonum`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_as_flonum(
    value: GerbilValueHandle,
    out: *mut GerbilFlonum,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    // SAFETY: raw predicate accepts a Scheme-object handle exported by Gerbil.
    if unsafe { gerbil_scheme_rust_scheme_object_is_flonum_raw(value) } == 0 {
        return GerbilStatus::InvalidValue;
    }

    // SAFETY: predicate above proved the value is a Scheme flonum.
    let projected = unsafe { gerbil_scheme_rust_scheme_object_flonum_value_raw(value) };
    // SAFETY: caller provided a non-null output pointer for one GerbilFlonum.
    unsafe {
        *out = GerbilFlonum(projected);
    }
    GerbilStatus::Ok
}
