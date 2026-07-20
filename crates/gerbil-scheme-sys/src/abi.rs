// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Raw, representation-stable types for the Gerbil Scheme native boundary.
//!
//! This crate intentionally contains no application schema and no policy
//! semantics.  Higher-level ownership and invocation APIs live in
//! `gerbil-scheme`.

//! Declares the stable raw C ABI shared by Rust and the native runtime shim.

use core::ffi::{c_char, c_void};

/// Stable ABI identifier for the first gerbil-scheme-rust binding family.
pub const GERBIL_SCHEME_RUST_ABI_ID: &[u8] = b"gerbil-scheme-rust.binding.v1\0";

/// ABI major version.  A mismatch must fail closed.
pub const GERBIL_SCHEME_RUST_ABI_VERSION: u32 = 1;

/// Repository-relative path to the public C header for this ABI.
pub const GERBIL_SCHEME_RUST_HEADER_PATH: &str = "include/gerbil_scheme_rust.h";

/// Public C header source compiled into the Rust ABI owner for drift detection.
pub const GERBIL_SCHEME_RUST_HEADER_SOURCE: &str =
    include_str!("../../../include/gerbil_scheme_rust.h");

/// Status returned by native binding entry points.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilStatus {
    /// The operation completed successfully.
    Ok = 0,
    /// A required pointer was null.
    NullPointer = 1,
    /// The requested ABI identifier or version is unsupported.
    AbiMismatch = 2,
    /// A value could not be converted at the checked boundary.
    InvalidValue = 3,
    /// The Gerbil runtime or compiler could not be invoked.
    RuntimeUnavailable = 4,
    /// A Rust panic was contained before it crossed the native boundary.
    Panic = 5,
    /// The process already owns an initialized Gerbil runtime.
    AlreadyInitialized = 6,
    /// Cleanup was requested without an initialized runtime.
    NotInitialized = 7,
    /// The runtime was cleaned up and cannot be restarted in this process.
    RuntimeFinalized = 8,
}

impl GerbilStatus {
    /// Returns the stable integer representation used by the C ABI.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }

    /// Decodes a status returned by the C ABI.
    ///
    /// Unknown values are preserved by returning `None`, allowing newer Gerbil
    /// runtimes to extend the status space without making this binding unsound.
    #[must_use]
    pub const fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => Some(Self::NullPointer),
            2 => Some(Self::AbiMismatch),
            3 => Some(Self::InvalidValue),
            4 => Some(Self::RuntimeUnavailable),
            5 => Some(Self::Panic),
            6 => Some(Self::AlreadyInitialized),
            7 => Some(Self::NotInitialized),
            8 => Some(Self::RuntimeFinalized),
            _ => None,
        }
    }
}

/// Borrowed UTF-8 bytes crossing the native boundary.
///
/// The owner of `ptr` must keep the bytes alive for the complete call.  The
/// callee must not retain or release the pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GerbilBorrowedUtf8 {
    /// Pointer to the first byte, or null only when `len` is zero.
    pub ptr: *const c_char,
    /// Number of bytes, excluding any optional trailing NUL.
    pub len: usize,
}

impl GerbilBorrowedUtf8 {
    /// Construct an empty borrowed UTF-8 value.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    /// Borrow a Rust UTF-8 string for the duration of one native call.
    #[must_use]
    pub fn from_utf8_str(value: &str) -> Self {
        if value.is_empty() {
            return Self::empty();
        }
        Self {
            ptr: value.as_ptr().cast(),
            len: value.len(),
        }
    }

    /// Returns whether this borrowed value is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Reborrow the pointed-to bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure `ptr` is either null with `len == 0`, or points
    /// to `len` initialized bytes that remain alive for the returned borrow.
    /// The bytes must be valid UTF-8 when the value is interpreted as a Scheme
    /// string.
    #[must_use]
    pub unsafe fn as_bytes<'a>(self) -> &'a [u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: guaranteed by the caller of this unsafe function.
        unsafe { std::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }

    /// Reborrow the pointed-to bytes as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`std::str::Utf8Error`] when the borrowed bytes are not valid
    /// UTF-8.
    ///
    /// # Safety
    ///
    /// The caller must uphold the same pointer and lifetime requirements as
    /// [`GerbilBorrowedUtf8::as_bytes`].
    pub unsafe fn as_str<'a>(self) -> Result<&'a str, std::str::Utf8Error> {
        // SAFETY: forwarded from this function's caller.
        std::str::from_utf8(unsafe { self.as_bytes() })
    }
}

impl From<&str> for GerbilBorrowedUtf8 {
    fn from(value: &str) -> Self {
        Self::from_utf8_str(value)
    }
}

/// Opaque runtime handle owned by the safe binding layer.
#[repr(C)]
#[derive(Debug)]
pub struct GerbilRuntimeOpaque {
    _private: [u8; 0],
    _pinned: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

/// Opaque value handle borrowed from a runtime.
///
/// Gambit represents the universal Scheme object (`scheme-object` in Gerbil
/// FFI) as a machine word. Keep the Rust ABI aligned with the public C header's
/// `uintptr_t` instead of modelling it as a dereferenceable pointer.
pub type GerbilValueHandle = usize;

/// Callback for a native binding that consumes one signed integer.
pub type GerbilI64Callback = unsafe extern "C" fn(i64, *mut c_void) -> GerbilStatus;

/// Machine-word fixnum value crossing the ABI by value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GerbilFixnum(pub isize);

/// Boolean value normalized to `0` or `1` at the ABI boundary.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GerbilBoolean(pub u8);

impl GerbilBoolean {
    /// ABI false value.
    pub const FALSE: Self = Self(0);
    /// ABI true value.
    pub const TRUE: Self = Self(1);

    /// Project a Rust boolean into the normalized ABI representation.
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value { Self::TRUE } else { Self::FALSE }
    }

    /// Interpret only the normalized false value as false.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.0 != 0
    }
}

/// Unicode scalar value encoded as a 32-bit code point.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GerbilChar(pub u32);

impl GerbilChar {
    /// Convert a Rust `char` to the ABI code point representation.
    #[must_use]
    pub const fn from_char(value: char) -> Self {
        Self(value as u32)
    }
}

impl TryFrom<GerbilChar> for char {
    type Error = ();

    fn try_from(value: GerbilChar) -> Result<Self, Self::Error> {
        char::from_u32(value.0).ok_or(())
    }
}

/// IEEE-754 double precision flonum value crossing the ABI by value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GerbilFlonum(pub f64);

/// Borrowed bytevector bytes crossing the native boundary.
///
/// The owner of `ptr` must keep the bytes alive for the complete call.  The
/// callee must not retain or release the pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GerbilBorrowedBytevector {
    /// Pointer to the first byte, or null only when `len` is zero.
    pub ptr: *const u8,
    /// Number of bytes available at `ptr`.
    pub len: usize,
}

impl GerbilBorrowedBytevector {
    /// Empty borrowed bytevector.
    pub const EMPTY: Self = Self {
        ptr: core::ptr::null(),
        len: 0,
    };

    /// Borrow a byte slice for the duration of a native call.
    #[must_use]
    pub const fn from_slice(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

impl Default for GerbilBorrowedBytevector {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Pair value represented as borrowed car/cdr handles.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GerbilPair {
    /// First value handle.
    pub car: GerbilValueHandle,
    /// Rest value handle.
    pub cdr: GerbilValueHandle,
}

/// Vector value represented as a borrowed contiguous handle slice.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GerbilBorrowedVector {
    /// Pointer to the first element handle, or null only when `len` is zero.
    pub ptr: *const GerbilValueHandle,
    /// Number of value handles available at `ptr`.
    pub len: usize,
}

impl GerbilBorrowedVector {
    /// Empty borrowed vector.
    pub const EMPTY: Self = Self {
        ptr: core::ptr::null(),
        len: 0,
    };

    /// Borrow a slice of value handles for the duration of a native call.
    #[must_use]
    pub const fn from_slice(values: &[GerbilValueHandle]) -> Self {
        Self {
            ptr: values.as_ptr(),
            len: values.len(),
        }
    }
}

impl Default for GerbilBorrowedVector {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Native procedure callback that receives one value handle and user context.
pub type GerbilProcedureCallback =
    unsafe extern "C" fn(GerbilValueHandle, *mut c_void) -> GerbilStatus;

/// Return whether a value handle is backed by a Scheme pair.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed type
/// classification exists.  It validates the handle/status boundary without
/// dereferencing unmanaged handles.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_value_is_pair(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe { checked_unbacked_value_predicate(value, out) }
}

/// Return an opaque value sentinel while the runtime is initialized.
///
/// The handle is intentionally Rust-owned and opaque.  Safe bindings may use it
/// to prove that a runtime-owned API path was used, but must not treat it as a
/// live Gambit/Gerbil object or infer pair/list traversal support.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_runtime_sentinel_value(
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    if out.is_null() {
        return GerbilStatus::NullPointer;
    }

    unsafe {
        *out = gerbil_scheme_rust_runtime_sentinel_value_sentinel().addr();
    }
    GerbilStatus::Ok
}

/// Return whether a value handle is backed by a Scheme list.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed list
/// classification exists.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_value_is_list(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe { checked_unbacked_value_predicate(value, out) }
}

/// Return whether a value handle is backed by Scheme null.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed null
/// classification exists.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilBoolean`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_value_is_null(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    unsafe { checked_unbacked_value_predicate(value, out) }
}

/// Project the car of a pair value into an output handle.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed pair
/// traversal exists.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_pair_car(
    value: GerbilValueHandle,
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe { checked_unbacked_value_handle_projection(value, out) }
}

/// Project the cdr of a pair value into an output handle.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed pair
/// traversal exists.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilValueHandle`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_pair_cdr(
    value: GerbilValueHandle,
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    unsafe { checked_unbacked_value_handle_projection(value, out) }
}

/// Project both car and cdr of a pair value.
///
/// This ABI entry point is intentionally fail-closed until runtime-backed pair
/// traversal exists.
///
/// # Safety
///
/// `out` must be non-null and valid for writing one [`GerbilPair`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_pair_parts(
    value: GerbilValueHandle,
    out: *mut GerbilPair,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    GerbilStatus::InvalidValue
}

unsafe fn checked_unbacked_value_predicate(
    value: GerbilValueHandle,
    out: *mut GerbilBoolean,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    // SAFETY: caller provided a non-null output pointer for one GerbilBoolean.
    unsafe {
        *out = GerbilBoolean::FALSE;
    }
    GerbilStatus::InvalidValue
}

fn gerbil_scheme_rust_runtime_sentinel_value_sentinel() -> *const u8 {
    static RUNTIME_SENTINEL_VALUE: u8 = 0;
    core::ptr::addr_of!(RUNTIME_SENTINEL_VALUE)
}

unsafe fn checked_unbacked_value_handle_projection(
    value: GerbilValueHandle,
    out: *mut GerbilValueHandle,
) -> GerbilStatus {
    if value == 0 || out.is_null() {
        return GerbilStatus::NullPointer;
    }
    // SAFETY: caller provided a non-null output pointer for one handle.
    unsafe {
        *out = 0;
    }
    GerbilStatus::InvalidValue
}

unsafe extern "C" {
    /// Initializes the process-global Gerbil runtime and binding module.
    ///
    /// # Safety
    ///
    /// This function is process-global and must be serialized with all other
    /// runtime lifecycle operations.
    pub fn gerbil_scheme_rust_runtime_init() -> i32;

    /// Cleans up the process-global Gerbil runtime.
    ///
    /// # Safety
    ///
    /// No Gerbil calls or values may remain live. The call must run on the
    /// thread that initialized the runtime.
    pub fn gerbil_scheme_rust_runtime_cleanup() -> i32;

    /// Returns the ABI version implemented by the loaded Gerbil module.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the Gerbil runtime is initialized for the
    /// current process and that the exporting module remains loaded.
    pub fn gerbil_scheme_rust_abi_version() -> u32;

    /// Returns its input through a C-only control path.
    ///
    /// # Safety
    ///
    /// The caller must link the native support library that owns this symbol.
    pub fn gerbil_scheme_rust_identity_i64(value: i64) -> i64;

    /// Scalar proof exported by `scheme/native.ss` through Gambit's official
    /// `c-define` C interface.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the Gerbil runtime is initialized for the
    /// current process and that the exporting module remains loaded.
    pub fn gerbil_scheme_rust_add_i64(left: i64, right: i64) -> i64;

    /// Scalar predicate exported by `scheme/native.ss` through Gambit's
    /// official `c-define` C interface.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the Gerbil runtime is initialized for the
    /// current process and that the exporting module remains loaded.
    pub fn gerbil_scheme_rust_is_even_i64(value: i64) -> i32;

    /// Three-way scalar comparison exported by `scheme/native.ss` through
    /// Gambit's official `c-define` C interface.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the Gerbil runtime is initialized for the
    /// current process and that the exporting module remains loaded.
    pub fn gerbil_scheme_rust_compare_i64(left: i64, right: i64) -> i32;
}
