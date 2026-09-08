//! Scheme exact-integer fixture, checked projection, and rooted-construction ABI.

use crate::abi::{
    GerbilBoolean, GerbilStatus, GerbilValueHandle, checked_scheme_object_fixture,
    checked_scheme_object_predicate,
};
use crate::abi_rooted_bytes::GerbilRootId;

unsafe extern "C" {
    fn gerbil_scheme_rust_fixture_exact_integer_large_positive_raw() -> GerbilValueHandle;
    fn gerbil_scheme_rust_fixture_exact_integer_large_negative_raw() -> GerbilValueHandle;
    fn gerbil_scheme_rust_scheme_object_is_exact_integer_raw(value: GerbilValueHandle) -> i32;
    fn gerbil_scheme_rust_scheme_object_exact_integer_fits_i64_raw(value: GerbilValueHandle)
    -> i32;
    fn gerbil_scheme_rust_scheme_object_exact_integer_fits_u64_raw(value: GerbilValueHandle)
    -> i32;
    fn gerbil_scheme_rust_scheme_object_exact_integer_i64_value_raw(
        value: GerbilValueHandle,
    ) -> i64;
    fn gerbil_scheme_rust_scheme_object_exact_integer_u64_value_raw(
        value: GerbilValueHandle,
    ) -> u64;
    fn gerbil_scheme_rust_i64_to_exact_integer_root_raw(value: i64) -> i64;
    fn gerbil_scheme_rust_u64_to_exact_integer_root_raw(value: u64) -> i64;
    fn gerbil_scheme_rust_root_is_exact_integer_raw(root: i64) -> i32;
    fn gerbil_scheme_rust_root_exact_integer_fits_i64_raw(root: i64) -> i32;
    fn gerbil_scheme_rust_root_exact_integer_fits_u64_raw(root: i64) -> i32;
    fn gerbil_scheme_rust_root_exact_integer_i64_value_raw(root: i64) -> i64;
    fn gerbil_scheme_rust_root_exact_integer_u64_value_raw(root: i64) -> u64;
}

/// Export a positive exact-integer fixture larger than `u64::MAX`.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_exact_integer_large_positive(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe {
        checked_scheme_object_fixture(
            out,
            gerbil_scheme_rust_fixture_exact_integer_large_positive_raw,
        )
    }
}

/// Export a negative exact-integer fixture smaller than `i64::MIN`.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_fixture_exact_integer_large_negative(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe {
        checked_scheme_object_fixture(
            out,
            gerbil_scheme_rust_fixture_exact_integer_large_negative_raw,
        )
    }
}

/// Check whether a Scheme-object export is an exact integer.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_is_exact_integer(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe {
        checked_scheme_object_predicate(
            value,
            out,
            gerbil_scheme_rust_scheme_object_is_exact_integer_raw,
        )
    }
}

/// Project an exact Scheme integer to `i64`, rejecting values outside the range.
///
/// # Safety
///
/// `value` must be a live Scheme-object export. `out` must be non-null and valid
/// for writing one `i64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_exact_integer_to_i64(
    value: GerbilValueHandle,
    out: *mut i64,
) -> GerbilStatus {
    unsafe {
        checked_scheme_exact_integer_projection(
            value,
            out,
            gerbil_scheme_rust_scheme_object_exact_integer_fits_i64_raw,
            gerbil_scheme_rust_scheme_object_exact_integer_i64_value_raw,
        )
    }
}

/// Project an exact Scheme integer to `u64`, rejecting negative or oversized values.
///
/// # Safety
///
/// `value` must be a live Scheme-object export. `out` must be non-null and valid
/// for writing one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_scheme_object_exact_integer_to_u64(
    value: GerbilValueHandle,
    out: *mut u64,
) -> GerbilStatus {
    unsafe {
        checked_scheme_exact_integer_projection(
            value,
            out,
            gerbil_scheme_rust_scheme_object_exact_integer_fits_u64_raw,
            gerbil_scheme_rust_scheme_object_exact_integer_u64_value_raw,
        )
    }
}

/// Construct a rooted Scheme exact integer from every `i64` bit pattern.
///
/// # Safety
///
/// The runtime must be initialized on its owner thread. `out` must be non-null
/// and valid for writing one [`GerbilRootId`]. The caller owns the returned root.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_i64_to_exact_integer_root(
    value: i64,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe {
        checked_exact_integer_root(out, || {
            gerbil_scheme_rust_i64_to_exact_integer_root_raw(value)
        })
    }
}

/// Construct a rooted Scheme exact integer from every `u64` bit pattern.
///
/// # Safety
///
/// The safety contract is identical to
/// [`gerbil_scheme_rust_i64_to_exact_integer_root`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_u64_to_exact_integer_root(
    value: u64,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe {
        checked_exact_integer_root(out, || {
            gerbil_scheme_rust_u64_to_exact_integer_root_raw(value)
        })
    }
}

/// Project a rooted exact Scheme integer to `i64` with range checking.
///
/// # Safety
///
/// `root` must identify a live exact integer owned by the caller. `out` must be
/// non-null and valid for writing one `i64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_exact_integer_to_i64(
    root: GerbilRootId,
    out: *mut i64,
) -> GerbilStatus {
    unsafe {
        checked_root_exact_integer_projection(
            root,
            out,
            gerbil_scheme_rust_root_exact_integer_fits_i64_raw,
            gerbil_scheme_rust_root_exact_integer_i64_value_raw,
        )
    }
}

/// Project a rooted exact Scheme integer to `u64` with range checking.
///
/// # Safety
///
/// The safety contract is identical to
/// [`gerbil_scheme_rust_root_exact_integer_to_i64`], except `out` writes `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_exact_integer_to_u64(
    root: GerbilRootId,
    out: *mut u64,
) -> GerbilStatus {
    unsafe {
        checked_root_exact_integer_projection(
            root,
            out,
            gerbil_scheme_rust_root_exact_integer_fits_u64_raw,
            gerbil_scheme_rust_root_exact_integer_u64_value_raw,
        )
    }
}

unsafe fn checked_scheme_exact_integer_projection<T: Copy>(
    value: GerbilValueHandle,
    out: *mut T,
    fits: unsafe extern "C" fn(GerbilValueHandle) -> i32,
    project: unsafe extern "C" fn(GerbilValueHandle) -> T,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    if unsafe { gerbil_scheme_rust_scheme_object_is_exact_integer_raw(value) } == 0
        || unsafe { fits(value) } == 0
    {
        return GerbilStatus::InvalidValue;
    }
    unsafe {
        *out = project(value);
    }
    GerbilStatus::Ok
}

unsafe fn checked_exact_integer_root(
    out: *mut GerbilRootId,
    create: impl FnOnce() -> i64,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let root = GerbilRootId(create());
    if !root.is_valid() || unsafe { gerbil_scheme_rust_root_is_exact_integer_raw(root.0) } == 0 {
        return GerbilStatus::InvalidValue;
    }
    unsafe {
        *out = root;
    }
    GerbilStatus::Ok
}

unsafe fn checked_root_exact_integer_projection<T: Copy>(
    root: GerbilRootId,
    out: *mut T,
    fits: unsafe extern "C" fn(i64) -> i32,
    project: unsafe extern "C" fn(i64) -> T,
) -> GerbilStatus {
    if !root.is_valid() || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    if unsafe { gerbil_scheme_rust_root_is_exact_integer_raw(root.0) } == 0
        || unsafe { fits(root.0) } == 0
    {
        return GerbilStatus::InvalidValue;
    }
    unsafe {
        *out = project(root.0);
    }
    GerbilStatus::Ok
}
