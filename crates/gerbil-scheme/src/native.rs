// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::{self, ThreadId};

use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_VERSION, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_runtime_cleanup, gerbil_scheme_rust_runtime_init,
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
            Self::IntegerOverflow { .. } => Some(GerbilStatus::InvalidValue),
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
        }
    }
}

impl std::error::Error for NativeError {}
