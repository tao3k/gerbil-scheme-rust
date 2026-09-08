//! Scheme-object fixnum fixture and projection ABI wrappers.

use crate::abi::{
    GerbilBoolean, GerbilFixnum, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate, gerbil_scheme_rust_fixture_fixnum_raw,
    gerbil_scheme_rust_scheme_object_fixnum_value_raw,
    gerbil_scheme_rust_scheme_object_is_fixnum_raw,
};

/// Export a Scheme fixnum fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_fixnum(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_fixnum_raw) }
}

/// Checks whether a Scheme-object export is a fixnum.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_fixnum(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    // SAFETY: raw predicate call is guarded by the shared checked predicate helper.
    unsafe {
        checked_scheme_object_predicate(value, out, gerbil_scheme_rust_scheme_object_is_fixnum_raw)
    }
}

/// Projects a Scheme-object fixnum into the fixnum ABI carrier.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilFixnum`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_as_fixnum(
    value: GerbilValueHandle,
    out: *mut GerbilFixnum,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    // SAFETY: raw predicate accepts a Scheme-object handle exported by Gerbil.
    if unsafe { gerbil_scheme_rust_scheme_object_is_fixnum_raw(value) } == 0 {
        return GerbilStatus::InvalidValue;
    }

    // SAFETY: predicate above proved the value is a fixnum.
    let projected = unsafe { gerbil_scheme_rust_scheme_object_fixnum_value_raw(value) };
    // SAFETY: caller provided a non-null output pointer for one GerbilFixnum.
    unsafe {
        *out = GerbilFixnum(projected);
    }
    GerbilStatus::Ok
}
