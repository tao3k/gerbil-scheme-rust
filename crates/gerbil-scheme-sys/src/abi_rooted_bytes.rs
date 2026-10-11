//! Rooted bytestring and bytevector conversion ABI wrappers.

use core::ffi::c_char;
use std::ffi::CString;

use super::abi::{
    GerbilBorrowedBytevector, GerbilBorrowedUtf8, GerbilChar, GerbilStatus, GerbilValueHandle,
};
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
    fn gerbil_scheme_rust_root_string_to_utf8_raw(root: i64) -> i64;
    fn gerbil_scheme_rust_root_utf8_to_string_raw(root: i64) -> i64;
    fn gerbil_scheme_rust_bytes_to_bytevector_root_raw(value: *const u8, len: u64) -> i64;
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
    fn gerbil_scheme_rust_root_bytevector_copy_raw(root: i64, out: *mut u8, len: u64) -> i64;
    fn gerbil_scheme_rust_root_release_raw(root: i64) -> i32;
}

/// Encode a rooted Scheme string with the checked Scheme-owned UTF-8 encoder.
/// The returned root owns independent bytes and must be released.
///
/// # Safety
/// The runtime must be initialized on its owner thread. `root` must identify
/// a live root; `out` must be null or writable for one token.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_string_to_utf8(
    root: GerbilRootId,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe { checked_root_conversion(root, out, gerbil_scheme_rust_root_string_to_utf8_raw) }
}

/// Decode a rooted bytevector with the official UTF-8 converter.
/// Invalid UTF-8 or a wrong root type leaves output untouched.
///
/// # Safety
/// The runtime must be initialized on its owner thread. `root` must identify
/// a live root; `out` must be null or writable for one token.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_utf8_to_string(
    root: GerbilRootId,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe { checked_root_conversion(root, out, gerbil_scheme_rust_root_utf8_to_string_raw) }
}

unsafe fn checked_root_conversion(
    root: GerbilRootId,
    out: *mut GerbilRootId,
    convert: unsafe extern "C" fn(i64) -> i64,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    let converted = GerbilRootId(unsafe { convert(root.0) });
    if !converted.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    unsafe { *out = converted };
    GerbilStatus::Ok
}

/// Copy borrowed binary input into a new rooted Scheme bytevector.
///
/// No text decoding or NUL termination is involved. On error `out` is untouched.
///
/// # Safety
///
/// The runtime must be initialized on its owner thread. Input must remain
/// readable and stable for the complete call; null is allowed only at length
/// zero. `out` must be null or writable for one root and must not overlap input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_bytes_to_bytevector_root(
    value: GerbilBorrowedBytevector,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    if out.is_null() || (value.len > 0 && value.ptr.is_null()) {
        return GerbilStatus::NullPointer;
    }
    let Ok(length) = u64::try_from(value.len) else {
        return GerbilStatus::InvalidValue;
    };
    if value.len > isize::MAX as usize {
        return GerbilStatus::InvalidValue;
    }
    let root =
        GerbilRootId(unsafe { gerbil_scheme_rust_bytes_to_bytevector_root_raw(value.ptr, length) });
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    unsafe { *out = root };
    GerbilStatus::Ok
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
    // Rust slices are bounded by isize::MAX, and CString additionally needs
    // one terminator byte. Reject impossible lengths before pointer access.
    if value.len >= isize::MAX as usize {
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

/// Copy a rooted Scheme bytevector into an exactly sized caller-owned buffer.
///
/// Zero-length bytevectors accept a null output pointer. On an invalid root,
/// wrong type, or length mismatch, the output remains untouched.
/// Success initializes all `len` bytes, including caller-owned uninitialized
/// storage. The output must not overlap Scheme-managed storage.
///
/// # Safety
///
/// The runtime must be initialized on its owner thread. `root` must identify
/// a live Scheme value, and `out` must be valid for writing `len` bytes when
/// `len` is nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_bytevector_copy(
    root: GerbilRootId,
    out: *mut u8,
    len: usize,
) -> GerbilStatus {
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    if len > 0 && out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let (Ok(raw_len), Ok(expected)) = (u64::try_from(len), i64::try_from(len)) else {
        return GerbilStatus::InvalidValue;
    };
    if unsafe { gerbil_scheme_rust_root_bytevector_copy_raw(root.0, out, raw_len) } == expected {
        GerbilStatus::Ok
    } else {
        GerbilStatus::InvalidValue
    }
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
