//! Integer and bytevector conversion ABI wrappers.

use super::abi::{GerbilStatus, GerbilValueHandle};
use super::abi_bytevector::{
    gerbil_scheme_rust_scheme_object_bytevector_length_raw,
    gerbil_scheme_rust_scheme_object_is_bytevector_raw,
};
use super::abi_rooted_bytes::{GerbilRootId, gerbil_scheme_rust_root_bytevector_length_raw};

/// Largest byte width representable by the public `u64` / `i64` conversion ABI.
pub const GERBIL_SCHEME_RUST_MAX_INTEGER_BYTES: u8 = 8;

/// Byte order accepted by the integer/bytevector conversion boundary.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilByteOrder {
    /// Most-significant byte first.
    Big = 0,
    /// Least-significant byte first.
    Little = 1,
    /// Native byte order of the compiled Rust/Gerbil runtime.
    Native = 2,
}

impl GerbilByteOrder {
    /// Stable integer code used by the C ABI.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }
}

unsafe extern "C" {
    fn gerbil_scheme_rust_bytevector_to_uint_raw(
        value: GerbilValueHandle,
        byte_order: i32,
        size: i64,
    ) -> u64;
    fn gerbil_scheme_rust_bytevector_to_sint_raw(
        value: GerbilValueHandle,
        byte_order: i32,
        size: i64,
    ) -> i64;
    fn gerbil_scheme_rust_root_bytevector_to_uint_raw(root: i64, byte_order: i32, size: i64)
    -> u64;
    fn gerbil_scheme_rust_root_bytevector_to_sint_raw(root: i64, byte_order: i32, size: i64)
    -> i64;
    fn gerbil_scheme_rust_uint_to_bytevector_root_raw(
        value: u64,
        byte_order: i32,
        size: i64,
    ) -> i64;
    fn gerbil_scheme_rust_sint_to_bytevector_root_raw(
        value: i64,
        byte_order: i32,
        size: i64,
    ) -> i64;
}

/// Decode an unsigned integer from the first `size` bytes of a Scheme bytevector.
///
/// A zero size follows Gerbil's official `u8vector->uint` behavior and decodes
/// to zero. Widths above eight bytes fail because the public result is `u64`.
///
/// # Safety
///
/// The Gerbil runtime/module must be initialized on the current owner thread.
/// `value` must be a live Scheme-object export. `out` must be null or valid for
/// writing one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_bytevector_to_uint(
    value: GerbilValueHandle,
    byte_order: i32,
    size: usize,
    out: *mut u64,
) -> GerbilStatus {
    let Some(byte_order) = normalized_byte_order(byte_order) else {
        return GerbilStatus::InvalidValue;
    };
    let size = match checked_scheme_bytevector_input(value, size, out) {
        Ok(size) => size,
        Err(status) => return status,
    };
    unsafe {
        *out = gerbil_scheme_rust_bytevector_to_uint_raw(value, byte_order, size);
    }
    GerbilStatus::Ok
}

/// Decode a signed two's-complement integer from a Scheme bytevector.
///
/// # Safety
///
/// The safety contract is identical to [`gerbil_scheme_rust_bytevector_to_uint`],
/// except `out` writes one `i64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_bytevector_to_sint(
    value: GerbilValueHandle,
    byte_order: i32,
    size: usize,
    out: *mut i64,
) -> GerbilStatus {
    let Some(byte_order) = normalized_byte_order(byte_order) else {
        return GerbilStatus::InvalidValue;
    };
    let size = match checked_scheme_bytevector_input(value, size, out) {
        Ok(size) => size,
        Err(status) => return status,
    };
    unsafe {
        *out = gerbil_scheme_rust_bytevector_to_sint_raw(value, byte_order, size);
    }
    GerbilStatus::Ok
}

/// Decode an unsigned integer from the first `size` bytes of a rooted bytevector.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `root` must identify a
/// live rooted bytevector. `out` must be null or valid for writing one `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_bytevector_to_uint(
    root: GerbilRootId,
    byte_order: i32,
    size: usize,
    out: *mut u64,
) -> GerbilStatus {
    let Some(byte_order) = normalized_byte_order(byte_order) else {
        return GerbilStatus::InvalidValue;
    };
    let size = match checked_rooted_bytevector_input(root, size, out) {
        Ok(size) => size,
        Err(status) => return status,
    };
    unsafe {
        *out = gerbil_scheme_rust_root_bytevector_to_uint_raw(root.0, byte_order, size);
    }
    GerbilStatus::Ok
}

/// Decode a signed two's-complement integer from a rooted bytevector.
///
/// # Safety
///
/// The safety contract is identical to
/// [`gerbil_scheme_rust_root_bytevector_to_uint`], except `out` writes one `i64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_root_bytevector_to_sint(
    root: GerbilRootId,
    byte_order: i32,
    size: usize,
    out: *mut i64,
) -> GerbilStatus {
    let Some(byte_order) = normalized_byte_order(byte_order) else {
        return GerbilStatus::InvalidValue;
    };
    let size = match checked_rooted_bytevector_input(root, size, out) {
        Ok(size) => size,
        Err(status) => return status,
    };
    unsafe {
        *out = gerbil_scheme_rust_root_bytevector_to_sint_raw(root.0, byte_order, size);
    }
    GerbilStatus::Ok
}

/// Encode an unsigned integer into a newly rooted Scheme bytevector.
///
/// This raw ABI preserves Gerbil's official fixed-width behavior: high bits are
/// truncated when `size` is too small. The safe crate exposes truncation only
/// through an explicit policy.
///
/// # Safety
///
/// The runtime must be initialized on the owner thread. `out` must be null or
/// valid for writing one [`GerbilRootId`]. The caller owns a successful root.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_uint_to_bytevector_root(
    value: u64,
    byte_order: i32,
    size: usize,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe {
        checked_integer_to_root(byte_order, size, out, |byte_order, size| {
            gerbil_scheme_rust_uint_to_bytevector_root_raw(value, byte_order, size)
        })
    }
}

/// Encode a signed integer as fixed-width two's complement in a rooted bytevector.
///
/// # Safety
///
/// The safety contract is identical to
/// [`gerbil_scheme_rust_uint_to_bytevector_root`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_sint_to_bytevector_root(
    value: i64,
    byte_order: i32,
    size: usize,
    out: *mut GerbilRootId,
) -> GerbilStatus {
    unsafe {
        checked_integer_to_root(byte_order, size, out, |byte_order, size| {
            gerbil_scheme_rust_sint_to_bytevector_root_raw(value, byte_order, size)
        })
    }
}

fn checked_scheme_bytevector_input<T>(
    value: GerbilValueHandle,
    size: usize,
    out: *mut T,
) -> Result<i64, GerbilStatus> {
    if value == 0 || out.is_null() {
        return Err(GerbilStatus::NullPointer);
    }
    let size = checked_size(size)?;
    if unsafe { gerbil_scheme_rust_scheme_object_is_bytevector_raw(value) } != 1 {
        return Err(GerbilStatus::InvalidValue);
    }
    let length = unsafe { gerbil_scheme_rust_scheme_object_bytevector_length_raw(value) };
    if length < size {
        return Err(GerbilStatus::InvalidValue);
    }
    Ok(size)
}

fn checked_rooted_bytevector_input<T>(
    root: GerbilRootId,
    size: usize,
    out: *mut T,
) -> Result<i64, GerbilStatus> {
    if !root.is_valid() || out.is_null() {
        return Err(GerbilStatus::NullPointer);
    }
    let size = checked_size(size)?;
    let length = unsafe { gerbil_scheme_rust_root_bytevector_length_raw(root.0) };
    if length < size {
        return Err(GerbilStatus::InvalidValue);
    }
    Ok(size)
}

unsafe fn checked_integer_to_root(
    byte_order: i32,
    size: usize,
    out: *mut GerbilRootId,
    encode: impl FnOnce(i32, i64) -> i64,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }
    let Some(byte_order) = normalized_byte_order(byte_order) else {
        return GerbilStatus::InvalidValue;
    };
    let Ok(size) = checked_size(size) else {
        return GerbilStatus::InvalidValue;
    };
    let root = GerbilRootId(encode(byte_order, size));
    if !root.is_valid() {
        return GerbilStatus::InvalidValue;
    }
    unsafe {
        *out = root;
    }
    GerbilStatus::Ok
}

fn checked_size(size: usize) -> Result<i64, GerbilStatus> {
    if size > usize::from(GERBIL_SCHEME_RUST_MAX_INTEGER_BYTES) {
        return Err(GerbilStatus::InvalidValue);
    }
    i64::try_from(size).map_err(|_| GerbilStatus::InvalidValue)
}

const fn normalized_byte_order(byte_order: i32) -> Option<i32> {
    match byte_order {
        0 | 1 => Some(byte_order),
        2 => {
            if cfg!(target_endian = "little") {
                Some(1)
            } else {
                Some(0)
            }
        }
        _ => None,
    }
}
