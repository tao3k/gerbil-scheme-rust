//! Scheme sentinel object ABI wrappers.

use super::abi::{
    GerbilBoolean, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate,
};

unsafe extern "C" {
    fn gerbil_scheme_rust_fixture_void_raw() -> GerbilValueHandle;
    fn gerbil_scheme_rust_scheme_object_is_void_raw(value: GerbilValueHandle) -> i32;
}

/// Export Scheme's `void` sentinel through a checked `GerbilValueHandle` out-parameter.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `out` must be either null,
/// which returns `GerbilStatus::NullPointer`, or valid for writing one
/// `GerbilValueHandle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_void(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe { checked_scheme_object_fixture(out, gerbil_scheme_rust_fixture_void_raw) }
}

/// Return whether a Scheme-object export is Scheme `void`.
///
/// # Safety
///
/// The caller must ensure that the Gerbil runtime and native module have been
/// initialized on the current runtime owner thread. `value` must be an opaque
/// Scheme object handle produced by the native bridge, and `out` must be either
/// null, which returns `GerbilStatus::NullPointer`, or valid for writing one
/// `GerbilBoolean`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_void(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe {
        checked_scheme_object_predicate(value, out, gerbil_scheme_rust_scheme_object_is_void_raw)
    }
}
