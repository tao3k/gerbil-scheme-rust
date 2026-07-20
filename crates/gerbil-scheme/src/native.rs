// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::{self, ThreadId};

use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_VERSION, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_identity_i64, gerbil_scheme_rust_runtime_cleanup,
    gerbil_scheme_rust_runtime_init,
};

static RUNTIME_LIFECYCLE: Mutex<()> = Mutex::new(());
const RUNTIME_NEVER_INITIALIZED: u8 = 0;
const RUNTIME_RUNNING: u8 = 1;
const RUNTIME_FINALIZED: u8 = 2;

static RUNTIME_STATE: AtomicU8 = AtomicU8::new(RUNTIME_NEVER_INITIALIZED);

/// Exclusive, thread-affine ownership of the in-process Gerbil runtime.
///
/// This handle is deliberately neither [`Send`] nor [`Sync`]. Its existence
/// proves that the process-global runtime was initialized successfully and that
/// the binding module reported the expected ABI version.
#[derive(Debug)]
pub struct GerbilRuntime {
    owner: ThreadId,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

/// Safe borrowed UTF-8 view for native Gerbil calls.
///
/// The Rust string owner keeps the bytes alive for the full borrow. The raw
/// [`gerbil_scheme_sys::GerbilBorrowedUtf8`] value may be passed to native code,
/// but the callee must not retain or free its pointer.
#[derive(Clone, Copy, Debug)]
pub struct GerbilUtf8<'a> {
    text: &'a str,
    abi: gerbil_scheme_sys::GerbilBorrowedUtf8,
}

impl<'a> GerbilUtf8<'a> {
    /// Borrow a Rust string as the stable native UTF-8 ABI shape.
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            abi: gerbil_scheme_sys::GerbilBorrowedUtf8::from(text),
        }
    }

    /// Return the Rust owner-side string view.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.text
    }

    /// Return the raw C ABI view for one native call.
    #[must_use]
    pub fn as_abi(self) -> gerbil_scheme_sys::GerbilBorrowedUtf8 {
        self.abi
    }

    /// Return the UTF-8 byte length.
    #[must_use]
    pub fn len(self) -> usize {
        self.abi.len
    }

    /// Return whether the string is empty.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.abi.len == 0
    }
}

impl<'a> From<&'a str> for GerbilUtf8<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

/// Runtime-borrowed opaque Gerbil value handle.
///
/// This wrapper is intentionally non-owning. It proves only that the raw handle
/// is non-null and is tied to a live [`GerbilRuntime`] borrow by the lifetime
/// parameter; it does not claim type, ownership, or GC reachability beyond that
/// borrow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilValue<'runtime> {
    raw: std::ptr::NonNull<std::ffi::c_void>,
    _runtime: PhantomData<&'runtime GerbilRuntime>,
}

impl<'runtime> GerbilValue<'runtime> {
    /// Wrap a raw runtime-borrowed value handle, rejecting null handles.
    pub fn from_raw(raw: gerbil_scheme_sys::GerbilValueHandle) -> Result<Self, NativeError> {
        let raw = std::ptr::NonNull::new(raw).ok_or(NativeError::Status {
            operation: "GerbilValue::from_raw",
            code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
        })?;

        Ok(Self {
            raw,
            _runtime: PhantomData,
        })
    }

    /// Return the raw borrowed value handle.
    #[must_use]
    pub fn as_raw(self) -> gerbil_scheme_sys::GerbilValueHandle {
        self.raw.as_ptr()
    }
}

/// Safe owner for a one-argument native i64 callback.
///
/// This wrapper accepts a plain Rust function pointer and exposes a borrowed C
/// ABI callback view for one native call. Panics are contained at the trampoline
/// and reported as [`gerbil_scheme_sys::GerbilStatus::Panic`].
#[derive(Clone, Copy, Debug)]
pub struct GerbilI64Callback {
    callback: fn(i64) -> gerbil_scheme_sys::GerbilStatus,
}

impl GerbilI64Callback {
    /// Build a native-safe callback wrapper from a Rust function pointer.
    #[must_use]
    pub fn new(callback: fn(i64) -> gerbil_scheme_sys::GerbilStatus) -> Self {
        Self { callback }
    }

    /// Borrow this callback as a C ABI callback/context pair.
    #[must_use]
    pub fn as_abi(&self) -> GerbilI64CallbackAbi<'_> {
        GerbilI64CallbackAbi {
            callback: gerbil_i64_callback_trampoline,
            context: std::ptr::from_ref(self).cast_mut().cast(),
            _callback: PhantomData,
        }
    }
}

/// Borrowed C ABI view of a [`GerbilI64Callback`].
#[derive(Clone, Copy, Debug)]
pub struct GerbilI64CallbackAbi<'callback> {
    callback: gerbil_scheme_sys::GerbilI64Callback,
    context: *mut std::ffi::c_void,
    _callback: PhantomData<&'callback GerbilI64Callback>,
}

impl<'callback> GerbilI64CallbackAbi<'callback> {
    /// Return the raw C callback function pointer.
    #[must_use]
    pub fn callback(self) -> gerbil_scheme_sys::GerbilI64Callback {
        self.callback
    }

    /// Return the raw borrowed callback context.
    #[must_use]
    pub fn context(self) -> *mut std::ffi::c_void {
        self.context
    }
}

unsafe extern "C" fn gerbil_i64_callback_trampoline(
    value: i64,
    context: *mut std::ffi::c_void,
) -> gerbil_scheme_sys::GerbilStatus {
    let Some(callback) = std::ptr::NonNull::<GerbilI64Callback>::new(context.cast()) else {
        return gerbil_scheme_sys::GerbilStatus::NullPointer;
    };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: `GerbilI64Callback::as_abi` creates this context from a live
        // shared borrow and ties the returned ABI view to that borrow. External
        // callers that manufacture a context bypass the safe API and receive
        // only panic/null containment here.
        unsafe { (callback.as_ref().callback)(value) }
    })) {
        Ok(status) => status,
        Err(_) => gerbil_scheme_sys::GerbilStatus::Panic,
    }
}

impl GerbilRuntime {
    /// Initializes the process-global Gerbil runtime and native binding module.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::AlreadyInitialized`] when another live handle
    /// owns the runtime, or a checked status/ABI error when native setup fails.
    pub fn initialize() -> Result<Self, NativeError> {
        let _lifecycle = RUNTIME_LIFECYCLE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        match RUNTIME_STATE.load(Ordering::Acquire) {
            RUNTIME_NEVER_INITIALIZED => {}
            RUNTIME_RUNNING => return Err(NativeError::AlreadyInitialized),
            RUNTIME_FINALIZED => return Err(NativeError::RuntimeFinalized),
            state => {
                return Err(NativeError::InvalidLifecycleState { state });
            }
        }

        // SAFETY: lifecycle operations are serialized by RUNTIME_LIFECYCLE and
        // no safe GerbilRuntime exists while RUNTIME_INITIALIZED is false.
        let status = unsafe { gerbil_scheme_rust_runtime_init() };
        if status != 0 {
            return Err(NativeError::Status {
                operation: "runtime initialization",
                code: status,
            });
        }

        // SAFETY: successful initialization loaded the binding module, and the
        // lifecycle lock prevents cleanup while the version is queried.
        let actual = unsafe { gerbil_scheme_rust_abi_version() };
        if actual != GERBIL_SCHEME_RUST_ABI_VERSION {
            // SAFETY: setup succeeded in this critical section and no binding
            // values or safe handles have escaped.
            let _ = unsafe { gerbil_scheme_rust_runtime_cleanup() };
            return Err(NativeError::AbiMismatch {
                expected: GERBIL_SCHEME_RUST_ABI_VERSION,
                actual,
            });
        }

        RUNTIME_STATE.store(RUNTIME_RUNNING, Ordering::Release);
        Ok(Self {
            owner: thread::current().id(),
            _not_send_or_sync: PhantomData,
        })
    }

    /// Returns the checked native ABI version.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread.
    pub fn abi_version(&self) -> Result<u32, NativeError> {
        self.check_thread()?;
        // SAFETY: self proves initialization and !Send keeps the call on the
        // owning thread after check_thread succeeds.
        Ok(unsafe { gerbil_scheme_rust_abi_version() })
    }

    /// Returns a signed 64-bit integer through the initialized Gerbil runtime.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread.
    pub fn identity_i64(&self, value: i64) -> Result<i64, NativeError> {
        self.check_thread()?;
        // SAFETY: self proves runtime/module lifetime; the scalar c-define ABI
        // accepts every i64 bit pattern and cannot retain borrowed Rust data.
        Ok(unsafe { gerbil_scheme_rust_identity_i64(value) })
    }

    /// Adds two signed 64-bit integers inside the initialized Gerbil runtime.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread.
    pub fn add_i64(&self, left: i64, right: i64) -> Result<i64, NativeError> {
        self.check_thread()?;
        if left.checked_add(right).is_none() {
            return Err(NativeError::IntegerOverflow { left, right });
        }
        // SAFETY: self proves runtime/module lifetime; the scalar c-define ABI
        // accepts every i64 bit pattern, the checked sum is representable, and
        // the call cannot retain borrowed Rust data.
        Ok(unsafe { gerbil_scheme_rust_add_i64(left, right) })
    }

    /// Tests whether a signed 64-bit integer is even inside Gerbil.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread.
    pub fn is_even_i64(&self, value: i64) -> Result<bool, NativeError> {
        self.check_thread()?;
        // SAFETY: self proves runtime/module lifetime; the scalar c-define ABI
        // accepts every i64 bit pattern and cannot retain borrowed Rust data.
        Ok(unsafe { gerbil_scheme_sys::gerbil_scheme_rust_is_even_i64(value) } != 0)
    }

    /// Compares two signed 64-bit integers inside Gerbil.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread, or [`NativeError::InvalidComparisonResult`] if the native module
    /// violates the ABI's three-way comparison contract.
    pub fn compare_i64(&self, left: i64, right: i64) -> Result<std::cmp::Ordering, NativeError> {
        self.check_thread()?;
        // SAFETY: self proves runtime/module lifetime; the scalar c-define ABI
        // accepts every i64 bit pattern and cannot retain borrowed Rust data.
        let code = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_compare_i64(left, right) };
        match code {
            -1 => Ok(std::cmp::Ordering::Less),
            0 => Ok(std::cmp::Ordering::Equal),
            1 => Ok(std::cmp::Ordering::Greater),
            code => Err(NativeError::InvalidComparisonResult { code }),
        }
    }

    fn check_thread(&self) -> Result<(), NativeError> {
        let actual = thread::current().id();
        if actual == self.owner {
            Ok(())
        } else {
            Err(NativeError::WrongThread {
                expected: self.owner,
                actual,
            })
        }
    }
}

impl Drop for GerbilRuntime {
    fn drop(&mut self) {
        let _lifecycle = RUNTIME_LIFECYCLE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        // SAFETY: GerbilRuntime is !Send, so safe Rust drops it on the owning
        // thread. Exclusive construction means this is the only safe handle.
        let status = unsafe { gerbil_scheme_rust_runtime_cleanup() };
        RUNTIME_STATE.store(RUNTIME_FINALIZED, Ordering::Release);
        debug_assert_eq!(status, 0, "Gerbil runtime cleanup failed: {status}");
    }
}

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

impl<T> NativeResult<T> {
    /// Constructs a successful native result.
    pub const fn ok(value: T) -> Self {
        Self { inner: Ok(value) }
    }

    /// Constructs a failed native result.
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
    pub const fn as_result(&self) -> Result<&T, &NativeError> {
        match &self.inner {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    /// Consumes the wrapper and returns the standard Rust result.
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
            Self::IntegerOverflow { .. } | Self::InvalidComparisonResult { .. } => {
                Some(GerbilStatus::InvalidValue)
            }
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
            Self::InvalidComparisonResult { code } => {
                write!(formatter, "invalid Gerbil i64 comparison result {code}")
            }
        }
    }
}

impl std::error::Error for NativeError {}
