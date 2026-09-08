//! Scheme-object character fixture and projection ABI wrappers.

use crate::abi::{
    GerbilBoolean, GerbilChar, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate, gerbil_scheme_rust_fixture_char_ascii_raw,
    gerbil_scheme_rust_fixture_char_bmp_raw, gerbil_scheme_rust_fixture_char_non_bmp_raw,
    gerbil_scheme_rust_scheme_object_char_value_raw, gerbil_scheme_rust_scheme_object_is_char_raw,
};

/// Export an ASCII Scheme character fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_char_ascii(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_char_ascii_raw) }
}

/// Export a BMP Scheme character fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_char_bmp(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_char_bmp_raw) }
}

/// Export a non-BMP Scheme character fixture through the initialized runtime.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_char_non_bmp(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    // SAFETY: FFI raw fixture call is guarded by the shared checked fixture helper.
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_char_non_bmp_raw) }
}

/// Checks whether a Scheme-object export is a character.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_char(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    // SAFETY: raw predicate call is guarded by the shared checked predicate helper.
    unsafe {
        checked_scheme_object_predicate(value, out, gerbil_scheme_rust_scheme_object_is_char_raw)
    }
}

/// Projects a Scheme-object character into the character ABI carrier.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilChar`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_as_char(
    value: GerbilValueHandle,
    out: *mut GerbilChar,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    // SAFETY: raw predicate accepts a Scheme-object handle exported by Gerbil.
    if unsafe { gerbil_scheme_rust_scheme_object_is_char_raw(value) } == 0 {
        return GerbilStatus::InvalidValue;
    }

    // SAFETY: predicate above proved the value is a Scheme character.
    let projected = unsafe { gerbil_scheme_rust_scheme_object_char_value_raw(value) };
    let Ok(projected) = u32::try_from(projected) else {
        return GerbilStatus::InvalidValue;
    };
    // SAFETY: caller provided a non-null output pointer for one GerbilChar.
    unsafe {
        *out = GerbilChar(projected);
    }
    GerbilStatus::Ok
}
