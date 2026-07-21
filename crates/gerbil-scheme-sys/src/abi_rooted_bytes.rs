//! Rooted bytestring and bytevector conversion ABI wrappers.

use core::ffi::c_char;
use std::ffi::CString;

use super::abi::{GerbilBorrowedUtf8, GerbilChar, GerbilStatus, GerbilValueHandle};
use super::abi_bytevector::gerbil_scheme_rust_scheme_object_is_bytevector_raw;

/// Positive Scheme root token owned by the native bridge.
///
/// A token keeps one converted Scheme object reachable until it is passed to
/// [`gerbil_scheme_rust_root_release`]. Zero is reserved for conversion
/// failures and is never a valid root.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRootId(pub i64);

impl GerbilRootId {
    /// Return whether this token can identify a live root.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 > 0
    }
}

unsafe extern "C" {
    fn gerbil_scheme_rust_bytevector_to_bytestring_root_raw(
        value: GerbilValueHandle,
        delimiter: i32,
    ) -> i64;
    fn gerbil_scheme_rust_bytestring_to_bytevector_root_raw(
        value: *const c_char,
        delimiter: i32,
    ) -> i64;
    fn gerbil_scheme_rust_root_string_length_raw(root: i64) -> i64;
    fn gerbil_scheme_rust_root_string_char_ref_raw(root: i64, index: i64) -> i32;
    pub(crate) fn gerbil_scheme_rust_root_bytevector_length_raw(root: i64) -> i64;
    fn gerbil_scheme_rust_root_bytevector_u8_ref_raw(root: i64, index: i64) -> i32;
    fn gerbil_scheme_rust_root_release_raw(root: i64) -> i32;
}

/// Convert a runtime-backed Scheme bytevector to a rooted uppercase hex string.
///
/// `delimiter` is `-1` for compact output or one Unicode scalar value to place
/// between bytes. The caller owns the returned root and must release it.
///
/// # Safety
///
/// The Gerbil runtime and native module must be initialized on the current
/// runtime owner thread. `value` must be a live Scheme-object export and `out`
/// must be null or valid for writing one [`GerbilRootId`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_bytevector_to_bytestring_root(
    value: GerbilValueHandle,
    delimiter: i32,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    if !valid_delimiter(delimiter)
        || unsafe { gerbil_scheme_rust_scheme_object_is_bytevector_raw(value) } != 1
    {
        return GerbilStatus::InvalidValue;
    }

    let root = GerbilRootId(unsafe {
        gerbil_scheme_rust_bytevector_to_bytestring_root_raw(value, delimiter)
    });
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }

    unsafe {
        *out = root;
    }
    GerbilStatus::Ok
}

/// Parse an ASCII hex bytestring into a rooted Scheme bytevector.
///
/// `delimiter` follows [`gerbil_scheme_rust_bytevector_to_bytestring_root`].
/// The caller owns the returned root and must release it.
///
/// # Safety
///
/// The Gerbil runtime and native module must be initialized on the current
/// runtime owner thread. `value` must satisfy the pointer/length contract of
/// [`GerbilBorrowedUtf8`], and `out` must be null or valid for writing one
/// [`GerbilRootId`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_bytestring_to_bytevector_root(
    value: GerbilBorrowedUtf8,
    delimiter: i32,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }
    if !valid_delimiter(delimiter) {
        return GerbilStatus::InvalidValue;
    }

    let bytes = if value.len == 0 {
        &[][..]
    } else {
        if value.ptr.is_null() {
            return GerbilStatus::NullPointer;
        }
        unsafe { std::slice::from_raw_parts(value.ptr.cast::<u8>(), value.len) }
    };
    if !bytes.is_ascii() {
        return GerbilStatus::InvalidValue;
    }
    let Ok(value) = CString::new(bytes) else {
        return GerbilStatus::InvalidValue;
    };

    let root = GerbilRootId(unsafe {
        gerbil_scheme_rust_bytestring_to_bytevector_root_raw(value.as_ptr(), delimiter)
    });
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }

    unsafe {
        *out = root;
    }
    GerbilStatus::Ok
}

/// Return the character length of a rooted Scheme string.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `root` must identify
/// a live rooted string and `out` must be null or valid for writing one length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_string_length(
    root: GerbilRootId,
    out: *mut usize,
) -> GerbilStatus {
    checked_root_length(root, out, gerbil_scheme_rust_root_string_length_raw)
}

/// Return one Unicode scalar from a rooted Scheme string.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `root` must identify
/// a live rooted string, `index` must be in bounds, and `out` must be null or
/// valid for writing one [`GerbilChar`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_string_char_ref(
    root: GerbilRootId,
    index: usize,
    out: *mut GerbilChar,
) -> GerbilStatus {
    if !root.is_valid() || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let Ok(index) = i64::try_from(index) else {
        return GerbilStatus::InvalidValue;
    };
    let code = unsafe { gerbil_scheme_rust_root_string_char_ref_raw(root.0, index) };
    let Ok(code) = u32::try_from(code) else {
        return GerbilStatus::InvalidValue;
    };
    let character = GerbilChar(code);
    if char::try_from(character).is_err() {
        return GerbilStatus::InvalidValue;
    }
    unsafe {
        *out = character;
    }
    GerbilStatus::Ok
}

/// Return the byte length of a rooted Scheme bytevector.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `root` must identify
/// a live rooted bytevector and `out` must be null or valid for one length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_bytevector_length(
    root: GerbilRootId,
    out: *mut usize,
) -> GerbilStatus {
    checked_root_length(root, out, gerbil_scheme_rust_root_bytevector_length_raw)
}

/// Return one byte from a rooted Scheme bytevector.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `root` must identify
/// a live rooted bytevector, `index` must be in bounds, and `out` must be null
/// or valid for writing one byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_bytevector_u8_ref(
    root: GerbilRootId,
    index: usize,
    out: *mut u8,
) -> GerbilStatus {
    if !root.is_valid() || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let Ok(index) = i64::try_from(index) else {
        return GerbilStatus::InvalidValue;
    };
    let byte = unsafe { gerbil_scheme_rust_root_bytevector_u8_ref_raw(root.0, index) };
    let Ok(byte) = u8::try_from(byte) else {
        return GerbilStatus::InvalidValue;
    };
    unsafe {
        *out = byte;
    }
    GerbilStatus::Ok
}

/// Release one rooted Scheme value.
///
/// # Safety
///
/// The runtime must be initialized on its owner thread and `root` must identify
/// one live root that has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_release(root: GerbilRootId) -> GerbilStatus {
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    if unsafe { gerbil_scheme_rust_root_release_raw(root.0) } == 1 {
        GerbilStatus::Ok
    } else {
        GerbilStatus::InvalidValue
    }
}

fn checked_root_length(
    root: GerbilRootId,
    out: *mut usize,
    length: unsafe extern "C" fn(i64) -> i64,
) -> GerbilStatus {
    if !root.is_valid() || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let length = unsafe { length(root.0) };
    let Ok(length) = usize::try_from(length) else {
        return GerbilStatus::InvalidValue;
    };
    unsafe {
        *out = length;
    }
    GerbilStatus::Ok
}

const fn valid_delimiter(delimiter: i32) -> bool {
    delimiter == -1
        || (delimiter >= 0
            && delimiter <= 0x10_ffff
            && !(delimiter >= 0xd800 && delimiter <= 0xdfff))
}
