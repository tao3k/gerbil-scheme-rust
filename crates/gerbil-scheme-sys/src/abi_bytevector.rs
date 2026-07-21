//! Scheme bytevector ABI wrappers.

use super::abi::{
    GerbilBoolean, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate,
};

unsafe extern "C" {
    fn gerbil_scheme_rust_fixture_bytevector_raw() -> GerbilValueHandle;
    pub(crate) fn gerbil_scheme_rust_scheme_object_is_bytevector_raw(
        value: GerbilValueHandle,
    ) -> i32;
    fn gerbil_scheme_rust_scheme_object_bytevector_length_raw(value: GerbilValueHandle) -> i64;
    fn gerbil_scheme_rust_scheme_object_bytevector_u8_ref_raw(
        value: GerbilValueHandle,
        index: i64,
    ) -> i32;
}

/// Export a Scheme bytevector fixture through a checked `GerbilValueHandle`.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `out` must be either null,
/// which returns `GerbilStatus::NullPointer`, or valid for writing one
/// `GerbilValueHandle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_bytevector(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_bytevector_raw) }
}

/// Return whether a Scheme-object export is a bytevector.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `value` must be an opaque
/// Scheme object handle produced by the native bridge, and `out` must be either
/// null, which returns `GerbilStatus::NullPointer`, or valid for writing one
/// `GerbilBoolean`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_bytevector(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe {
        checked_scheme_object_predicate(
            value,
            out,
            gerbil_scheme_rust_scheme_object_is_bytevector_raw,
        )
    }
}

/// Project a Scheme bytevector length into `out`.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `value` must be an opaque
/// Scheme object handle produced by the native bridge, and `out` must be either
/// null, which returns `GerbilStatus::NullPointer`, or valid for writing one
/// byte length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_bytevector_length(
    value: GerbilValueHandle,
    out: *mut usize,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }

    let is_bytevector = unsafe { gerbil_scheme_rust_scheme_object_is_bytevector_raw(value) };
    if is_bytevector != 1 {
        return GerbilStatus::InvalidValue;
    }

    let len = unsafe { gerbil_scheme_rust_scheme_object_bytevector_length_raw(value) };
    let Ok(len) = usize::try_from(len) else {
        return GerbilStatus::InvalidValue;
    };

    unsafe {
        *out = len;
    }
    GerbilStatus::Ok
}

/// Project one byte from a Scheme bytevector into `out`.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `value` must be an opaque
/// Scheme object handle produced by the native bridge, `index` must name the
/// byte to project, and `out` must be either null, which returns
/// `GerbilStatus::NullPointer`, or valid for writing one byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_bytevector_u8_ref(
    value: GerbilValueHandle,
    index: usize,
    out: *mut u8,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }

    let Ok(index) = i64::try_from(index) else {
        return GerbilStatus::InvalidValue;
    };
    let byte = unsafe { gerbil_scheme_rust_scheme_object_bytevector_u8_ref_raw(value, index) };
    let Ok(byte) = u8::try_from(byte) else {
        return GerbilStatus::InvalidValue;
    };

    unsafe {
        *out = byte;
    }
    GerbilStatus::Ok
}
