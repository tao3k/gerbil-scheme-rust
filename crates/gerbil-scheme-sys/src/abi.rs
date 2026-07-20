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
pub type GerbilValueHandle = *mut c_void;

/// Callback for a native binding that consumes one signed integer.
pub type GerbilI64Callback = unsafe extern "C" fn(i64, *mut c_void) -> GerbilStatus;

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
