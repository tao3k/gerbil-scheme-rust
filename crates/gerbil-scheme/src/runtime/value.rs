//! Interpret borrowed Scheme values and preserve typed native failures.

use super::{ExactIntegerTarget, GerbilRuntime};
use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::thread::ThreadId;

/// Failure at the safe in-process Gerbil boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeError {
    /// Another live safe handle already owns the process-global runtime.
    AlreadyInitialized,
    /// Gambit cleanup ran and this process cannot initialize the runtime again.
    RuntimeFinalized,
    /// Internal lifecycle state was corrupted.
    InvalidLifecycleState {
        /// Unexpected state byte.
        state: u8,
    },
    /// A native lifecycle operation returned a non-zero status.
    Status {
        /// Operation that failed.
        operation: &'static str,
        /// Stable native status code.
        code: i32,
    },
    /// The loaded binding module does not implement the expected ABI.
    AbiMismatch {
        /// Version compiled into the Rust crate.
        expected: u32,
        /// Version reported by the loaded Gerbil module.
        actual: u32,
    },
    /// A runtime method was called from a thread other than its owner.
    WrongThread {
        /// Thread that initialized the runtime.
        expected: ThreadId,
        /// Calling thread.
        actual: ThreadId,
    },
    /// Integer projection would exceed the declared C ABI result type.
    IntegerOverflow {
        /// Left operand.
        left: i64,
        /// Right operand.
        right: i64,
    },
    /// An unsigned value does not fit a checked fixed-width encoding.
    UnsignedIntegerWidth {
        /// Value that would lose high bits.
        value: u64,
        /// Requested width in bytes.
        width: u8,
    },
    /// A signed value does not fit a checked fixed-width encoding.
    SignedIntegerWidth {
        /// Value that cannot be represented at the requested width.
        value: i64,
        /// Requested width in bytes.
        width: u8,
    },
    /// A Scheme exact integer cannot be represented by the requested Rust target.
    ExactIntegerOutOfRange {
        /// Checked Rust machine target.
        target: ExactIntegerTarget,
    },
    /// A three-way comparison returned a value outside `-1`, `0`, and `1`.
    InvalidComparisonResult {
        /// Unexpected result returned by the native binding.
        code: i32,
    },
}

/// Result wrapper for safe in-process Gerbil calls.
///
/// This keeps the Rust-facing API aligned with the native surface shape:
/// success projects to `GerbilStatus::Ok`, known native failures project to
/// their stable status, and unknown status codes stay preserved inside
/// [`NativeError::Status`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeResult<T> {
    inner: Result<T, NativeError>,
}

/// Safe by-value Scheme scalar surface.
///
/// This enum is intentionally limited to values that can cross the C ABI by
/// value without claiming runtime allocation, GC rooting, or object ownership.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SchemeScalar {
    /// Gerbil fixnum represented as a machine word.
    Fixnum(isize),
    /// Gerbil boolean.
    Boolean(bool),
    /// Gerbil character represented as a Unicode scalar.
    Char(char),
    /// Gerbil flonum represented as IEEE-754 double precision.
    Flonum(f64),
}

macro_rules! define_handle_backed_scheme_view {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name<'runtime> {
            raw: NonZeroUsize,
            _runtime: PhantomData<&'runtime GerbilRuntime>,
        }

        impl<'runtime> $name<'runtime> {
            /// Wrap a non-zero runtime-owned handle.
            ///
            /// This proves only non-zero identity. It does not inspect,
            /// dereference, allocate, free, root, or otherwise claim ownership
            /// of the Gerbil object behind the handle.
            #[must_use]
            pub fn from_raw(raw: gerbil_scheme_sys::GerbilValueHandle) -> Option<Self> {
                NonZeroUsize::new(raw).map(|raw| Self {
                    raw,
                    _runtime: PhantomData,
                })
            }

            /// Return the borrowed raw handle without dereferencing it.
            #[must_use]
            pub const fn as_raw(self) -> gerbil_scheme_sys::GerbilValueHandle {
                self.raw.get()
            }
        }
    };
}

define_handle_backed_scheme_view!(
    /// Runtime-borrowed handle-backed Scheme symbol view.
    ///
    /// This view does not allocate or intern a symbol; it only preserves the
    /// identity of a non-zero runtime-owned value handle that another checked
    /// boundary has already classified as a symbol.
    SchemeSymbol
);

define_handle_backed_scheme_view!(
    /// Runtime-borrowed handle-backed Scheme keyword view.
    ///
    /// This view does not allocate or intern a keyword; it only preserves the
    /// identity of a non-zero runtime-owned value handle that another checked
    /// boundary has already classified as a keyword.
    SchemeKeyword
);

define_handle_backed_scheme_view!(
    /// Runtime-borrowed handle-backed Scheme pair view.
    ///
    /// This view does not expose car/cdr traversal. Pair traversal must first be
    /// backed by explicit sys ABI functions that own the status/error boundary.
    SchemePair
);

define_handle_backed_scheme_view!(
    /// Runtime-borrowed handle-backed Scheme list view.
    ///
    /// This view does not traverse the list. List traversal must first be backed
    /// by explicit sys ABI functions that own the status/error boundary.
    SchemeList
);

impl SchemeScalar {
    /// Project this scalar to its raw fixnum ABI representation when possible.
    #[must_use]
    pub const fn as_fixnum_abi(self) -> Option<gerbil_scheme_sys::GerbilFixnum> {
        match self {
            Self::Fixnum(value) => Some(gerbil_scheme_sys::GerbilFixnum(value)),
            Self::Boolean(_) | Self::Char(_) | Self::Flonum(_) => None,
        }
    }

    /// Project this scalar to its raw boolean ABI representation when possible.
    #[must_use]
    pub const fn as_boolean_abi(self) -> Option<gerbil_scheme_sys::GerbilBoolean> {
        match self {
            Self::Boolean(value) => Some(gerbil_scheme_sys::GerbilBoolean::from_bool(value)),
            Self::Fixnum(_) | Self::Char(_) | Self::Flonum(_) => None,
        }
    }

    /// Project this scalar to its raw character ABI representation when possible.
    #[must_use]
    pub const fn as_char_abi(self) -> Option<gerbil_scheme_sys::GerbilChar> {
        match self {
            Self::Char(value) => Some(gerbil_scheme_sys::GerbilChar::from_char(value)),
            Self::Fixnum(_) | Self::Boolean(_) | Self::Flonum(_) => None,
        }
    }

    /// Project this scalar to its raw flonum ABI representation when possible.
    #[must_use]
    pub const fn as_flonum_abi(self) -> Option<gerbil_scheme_sys::GerbilFlonum> {
        match self {
            Self::Flonum(value) => Some(gerbil_scheme_sys::GerbilFlonum(value)),
            Self::Fixnum(_) | Self::Boolean(_) | Self::Char(_) => None,
        }
    }
}

impl From<isize> for SchemeScalar {
    fn from(value: isize) -> Self {
        Self::Fixnum(value)
    }
}

impl From<bool> for SchemeScalar {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<char> for SchemeScalar {
    fn from(value: char) -> Self {
        Self::Char(value)
    }
}

impl From<f64> for SchemeScalar {
    fn from(value: f64) -> Self {
        Self::Flonum(value)
    }
}

/// Safe borrowed bytevector view for native Gerbil calls.
///
/// The Rust slice owner keeps the bytes alive for the full borrow. The native
/// callee must not retain or free the pointer.
#[derive(Clone, Copy, Debug)]
pub struct SchemeBorrowedBytevector<'a> {
    bytes: &'a [u8],
    abi: gerbil_scheme_sys::GerbilBorrowedBytevector,
}

impl<'a> SchemeBorrowedBytevector<'a> {
    /// Borrow a byte slice for the duration of a native call.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            abi: gerbil_scheme_sys::GerbilBorrowedBytevector::from_slice(bytes),
        }
    }

    /// Return the original borrowed bytes.
    #[must_use]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Return the C ABI projection for this borrow.
    #[must_use]
    pub const fn as_abi(self) -> gerbil_scheme_sys::GerbilBorrowedBytevector {
        self.abi
    }
}

/// Safe borrowed vector view for native Gerbil value handles.
///
/// This does not root or own the values. It only preserves the handle slice
/// shape for a native call whose runtime ownership is managed elsewhere.
#[derive(Clone, Copy, Debug)]
pub struct SchemeBorrowedVector<'a> {
    values: &'a [gerbil_scheme_sys::GerbilValueHandle],
    abi: gerbil_scheme_sys::GerbilBorrowedVector,
}

impl<'a> SchemeBorrowedVector<'a> {
    /// Borrow a value-handle slice for the duration of a native call.
    #[must_use]
    pub const fn new(values: &'a [gerbil_scheme_sys::GerbilValueHandle]) -> Self {
        Self {
            values,
            abi: gerbil_scheme_sys::GerbilBorrowedVector::from_slice(values),
        }
    }

    /// Return the original borrowed value handles.
    #[must_use]
    pub const fn as_values(self) -> &'a [gerbil_scheme_sys::GerbilValueHandle] {
        self.values
    }

    /// Return the C ABI projection for this borrow.
    #[must_use]
    pub const fn as_abi(self) -> gerbil_scheme_sys::GerbilBorrowedVector {
        self.abi
    }
}

impl<T> NativeResult<T> {
    /// Constructs a successful native result.
    pub const fn ok(value: T) -> Self {
        Self { inner: Ok(value) }
    }

    /// Constructs a failed native result.
    #[must_use]
    pub const fn err(error: NativeError) -> Self {
        Self { inner: Err(error) }
    }

    /// Wraps a standard Rust result at the native boundary.
    pub const fn from_result(inner: Result<T, NativeError>) -> Self {
        Self { inner }
    }

    /// Returns true when the native call succeeded.
    pub const fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }

    /// Returns true when the native call failed.
    pub const fn is_err(&self) -> bool {
        self.inner.is_err()
    }

    /// Projects the result to the stable native status surface.
    #[must_use]
    pub const fn status(&self) -> Option<gerbil_scheme_sys::GerbilStatus> {
        match &self.inner {
            Ok(_) => Some(gerbil_scheme_sys::GerbilStatus::Ok),
            Err(error) => error.status(),
        }
    }

    /// Borrows the wrapped Rust result.
    ///
    /// # Errors
    ///
    /// Returns a borrowed [`NativeError`] when the wrapped native call failed.
    pub const fn as_result(&self) -> Result<&T, &NativeError> {
        match &self.inner {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    /// Consumes the wrapper and returns the standard Rust result.
    ///
    /// # Errors
    ///
    /// Returns the owned [`NativeError`] when the wrapped native call failed.
    pub fn into_result(self) -> Result<T, NativeError> {
        self.inner
    }
}

impl<T> From<Result<T, NativeError>> for NativeResult<T> {
    fn from(inner: Result<T, NativeError>) -> Self {
        Self::from_result(inner)
    }
}

impl<T> From<NativeResult<T>> for Result<T, NativeError> {
    fn from(result: NativeResult<T>) -> Self {
        result.into_result()
    }
}

impl NativeError {
    /// Returns the stable ABI status represented by this error, when one exists.
    ///
    /// Raw status codes from newer runtimes remain available through the
    /// [`NativeError::Status`] variant even when this binding cannot decode
    /// them yet.
    #[must_use]
    pub const fn status(&self) -> Option<gerbil_scheme_sys::GerbilStatus> {
        use gerbil_scheme_sys::GerbilStatus;

        match self {
            Self::AlreadyInitialized => Some(GerbilStatus::AlreadyInitialized),
            Self::RuntimeFinalized => Some(GerbilStatus::RuntimeFinalized),
            Self::Status { code, .. } => GerbilStatus::from_code(*code),
            Self::AbiMismatch { .. } => Some(GerbilStatus::AbiMismatch),
            Self::IntegerOverflow { .. }
            | Self::UnsignedIntegerWidth { .. }
            | Self::SignedIntegerWidth { .. }
            | Self::ExactIntegerOutOfRange { .. }
            | Self::InvalidComparisonResult { .. } => Some(GerbilStatus::InvalidValue),
            Self::InvalidLifecycleState { .. } | Self::WrongThread { .. } => None,
        }
    }
}

impl fmt::Display for NativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => {
                formatter.write_str("Gerbil runtime is already initialized")
            }
            Self::RuntimeFinalized => formatter
                .write_str("Gerbil runtime was finalized and cannot be restarted in this process"),
            Self::InvalidLifecycleState { state } => {
                write!(formatter, "invalid Gerbil runtime lifecycle state {state}")
            }
            Self::Status { operation, code } => {
                write!(formatter, "{operation} failed with native status {code}")
            }
            Self::AbiMismatch { expected, actual } => {
                write!(
                    formatter,
                    "Gerbil ABI mismatch: expected {expected}, got {actual}"
                )
            }
            Self::WrongThread { expected, actual } => write!(
                formatter,
                "Gerbil runtime thread mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::IntegerOverflow { left, right } => {
                write!(formatter, "Gerbil i64 addition overflows: {left} + {right}")
            }
            Self::UnsignedIntegerWidth { value, width } => write!(
                formatter,
                "unsigned integer {value} does not fit a {width}-byte encoding"
            ),
            Self::SignedIntegerWidth { value, width } => write!(
                formatter,
                "signed integer {value} does not fit a {width}-byte encoding"
            ),
            Self::ExactIntegerOutOfRange { target } => {
                write!(
                    formatter,
                    "Scheme exact integer does not fit Rust {target:?}"
                )
            }
            Self::InvalidComparisonResult { code } => {
                write!(formatter, "invalid Gerbil i64 comparison result {code}")
            }
        }
    }
}

impl std::error::Error for NativeError {}
