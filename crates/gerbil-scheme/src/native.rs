// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme_sys::{
    gerbil_scheme_rust_fixture_false, gerbil_scheme_rust_fixture_fixnum,
    gerbil_scheme_rust_fixture_improper_list, gerbil_scheme_rust_fixture_pair,
    gerbil_scheme_rust_fixture_proper_list, gerbil_scheme_rust_fixture_true,
    gerbil_scheme_rust_scheme_object_as_boolean, gerbil_scheme_rust_scheme_object_as_fixnum,
    gerbil_scheme_rust_scheme_object_is_boolean, gerbil_scheme_rust_scheme_object_is_fixnum,
    gerbil_scheme_rust_scheme_object_is_list, gerbil_scheme_rust_scheme_object_is_pair,
};

use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::{self, ThreadId};

use gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_is_null;
use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_VERSION, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_fixture_null, gerbil_scheme_rust_identity_i64,
    gerbil_scheme_rust_runtime_cleanup, gerbil_scheme_rust_runtime_init,
    gerbil_scheme_rust_runtime_sentinel_value,
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

/// Provenance attached to a borrowed Gerbil value handle.
///
/// This is deliberately narrower than "non-zero word".  Runtime-backed
/// Scheme predicates and traversal must only use handles whose provenance is
/// produced by an initialized runtime/export path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilValueProvenance {
    /// A raw non-zero handle supplied by the caller.
    ///
    /// This keeps tests and FFI boundaries fail-closed: the pointer is not
    /// trusted as a live Gambit/Gerbil object.
    UntrustedRaw,
    /// A Rust-owned sentinel handle produced through an initialized runtime API path.
    ///
    /// This proves the handle was produced while the owning [`GerbilRuntime`]
    /// was alive. It is not a live Gambit/Gerbil object and does not imply type,
    /// GC rooting, or traversal safety.
    RuntimeSentinel,
    /// A borrowed Scheme object produced by the initialized Gerbil native module.
    ///
    /// This proves the handle came from a Gerbil `scheme-object` export while
    /// the owning [`GerbilRuntime`] was alive. It is still borrowed and
    /// unrooted, so traversal and retention remain gated by later APIs.
    SchemeObjectExport,
}

/// Runtime-borrowed opaque Gerbil value handle.
///
/// This wrapper is intentionally non-owning. It proves only that the raw handle
/// is non-zero. Runtime provenance is tracked explicitly by
/// [`GerbilValueProvenance`]; a caller-created raw handle is not enough to
/// claim type, ownership, GC reachability, or validity as a Gambit object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilValue<'runtime> {
    raw: NonZeroUsize,
    provenance: GerbilValueProvenance,
    _runtime: PhantomData<&'runtime GerbilRuntime>,
}

/// Borrowed pair parts projected from a runtime-backed pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemePairParts<'runtime> {
    /// Projected car value.
    pub car: GerbilValue<'runtime>,
    /// Projected cdr value.
    pub cdr: GerbilValue<'runtime>,
}

impl<'runtime> GerbilValue<'runtime> {
    /// Return whether this value is known to be a pair.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_pair(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_pair",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_pair,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_pair",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_pair,
                )
            }
        }
    }

    /// Return whether this value is known to be a list.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_list(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_list",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_list,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_list",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_list,
                )
            }
        }
    }

    /// Return whether this value is known to be Scheme null.
    ///
    /// Until the sys ABI is backed by runtime classification, this returns a
    /// fail-closed [`NativeError::Status`] instead of guessing.
    #[must_use]
    pub fn is_null(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_null",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_null,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                checked_native_predicate(
                    "gerbil_scheme_rust_value_is_null",
                    self.raw.get(),
                    gerbil_scheme_sys::gerbil_scheme_rust_value_is_null,
                )
            }
        }
    }

    /// Project this value's car if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    /// Checks whether this value is a Scheme boolean.
    ///
    /// This only succeeds for values exported by the initialized Gerbil runtime.
    #[must_use]
    pub fn is_boolean(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_boolean",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_boolean,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_boolean",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme boolean.
    ///
    /// This only succeeds for Scheme-object exports that satisfy `boolean?`.
    #[must_use]
    pub fn as_boolean(self) -> NativeResult<bool> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_boolean",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilBoolean::from_bool(false);
        // SAFETY: `out` is a valid output slot for one GerbilBoolean.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_boolean(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_boolean",
                code: status as i32,
            });
        }
        NativeResult::ok(out.as_bool())
    }

    /// Returns whether this value is a Scheme fixnum.
    ///
    /// This only succeeds for Scheme-object exports; untrusted raw handles and
    /// runtime sentinels fail closed with `InvalidValue`.
    #[must_use]
    pub fn is_fixnum(self) -> NativeResult<bool> {
        match self.provenance {
            GerbilValueProvenance::SchemeObjectExport => checked_native_predicate(
                "gerbil_scheme_rust_scheme_object_is_fixnum",
                self.raw.get(),
                gerbil_scheme_rust_scheme_object_is_fixnum,
            ),
            GerbilValueProvenance::UntrustedRaw | GerbilValueProvenance::RuntimeSentinel => {
                NativeResult::err(NativeError::Status {
                    operation: "gerbil_scheme_rust_scheme_object_is_fixnum",
                    code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
                })
            }
        }
    }

    /// Projects this value as a Scheme fixnum.
    ///
    /// This intentionally covers only Gerbil fixnums. Bignums and other exact
    /// integer objects must use a later, explicitly versioned projection path.
    #[must_use]
    pub fn as_fixnum(self) -> NativeResult<isize> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_fixnum",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut out = gerbil_scheme_sys::GerbilFixnum::default();
        // SAFETY: `out` is a valid output slot for one GerbilFixnum.
        let status =
            unsafe { gerbil_scheme_rust_scheme_object_as_fixnum(self.raw.get(), &raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_scheme_object_as_fixnum",
                code: status as i32,
            });
        }
        NativeResult::ok(out.0)
    }

    /// Projects this value as a Scheme fixnum widened to `i64`.
    #[must_use]
    pub fn as_fixnum_i64(self) -> NativeResult<i64> {
        match self.as_fixnum().as_result() {
            Ok(value) => NativeResult::ok(*value as i64),
            Err(error) => NativeResult::err(*error),
        }
    }

    /// Project this value's car if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_car(self) -> NativeResult<Self> {
        checked_scheme_object_projection(
            "gerbil_scheme_rust_pair_car",
            self,
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_car,
        )
    }

    /// Project this value's cdr if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_cdr(self) -> NativeResult<Self> {
        checked_scheme_object_projection(
            "gerbil_scheme_rust_pair_cdr",
            self,
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_cdr,
        )
    }

    /// Project this value's pair parts if it is backed by a pair.
    ///
    /// This delegates to the sys ABI and only succeeds for Scheme-object
    /// exports.
    #[must_use]
    pub fn pair_parts(self) -> NativeResult<SchemePairParts<'runtime>> {
        if self.provenance != GerbilValueProvenance::SchemeObjectExport {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts",
                code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
            });
        }

        let mut pair = gerbil_scheme_sys::GerbilPair { car: 0, cdr: 0 };
        // SAFETY: `pair` is a valid output slot for one GerbilPair.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_scheme_object_pair_parts(
                self.raw.get(),
                &raw mut pair,
            )
        };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts",
                code: status as i32,
            });
        }
        let Some(car) = value_from_native_handle_with_provenance(
            pair.car,
            GerbilValueProvenance::SchemeObjectExport,
        ) else {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts.car",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            });
        };
        let Some(cdr) = value_from_native_handle_with_provenance(
            pair.cdr,
            GerbilValueProvenance::SchemeObjectExport,
        ) else {
            return NativeResult::err(NativeError::Status {
                operation: "gerbil_scheme_rust_pair_parts.cdr",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            });
        };
        NativeResult::ok(SchemePairParts { car, cdr })
    }
}

type NativePredicate = unsafe extern "C" fn(
    gerbil_scheme_sys::GerbilValueHandle,
    *mut gerbil_scheme_sys::GerbilBoolean,
) -> gerbil_scheme_sys::GerbilStatus;

type NativeValueProjection = unsafe extern "C" fn(
    gerbil_scheme_sys::GerbilValueHandle,
    *mut gerbil_scheme_sys::GerbilValueHandle,
) -> gerbil_scheme_sys::GerbilStatus;

fn checked_native_predicate(
    operation: &'static str,
    value: gerbil_scheme_sys::GerbilValueHandle,
    predicate: NativePredicate,
) -> NativeResult<bool> {
    let mut out = gerbil_scheme_sys::GerbilBoolean::FALSE;
    // SAFETY: `out` is a valid output slot for one GerbilBoolean.
    let status = unsafe { predicate(value, &raw mut out) };
    if status == gerbil_scheme_sys::GerbilStatus::Ok {
        NativeResult::ok(out.as_bool())
    } else {
        NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        })
    }
}

fn checked_scheme_object_projection<'runtime>(
    operation: &'static str,
    value: GerbilValue<'runtime>,
    projection: NativeValueProjection,
) -> NativeResult<GerbilValue<'runtime>> {
    if value.provenance != GerbilValueProvenance::SchemeObjectExport {
        return NativeResult::err(NativeError::Status {
            operation,
            code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
        });
    }

    let mut out = 0;
    // SAFETY: `out` is a valid output slot for one Gerbil value handle.
    let status = unsafe { projection(value.raw.get(), &raw mut out) };
    if status != gerbil_scheme_sys::GerbilStatus::Ok {
        return NativeResult::err(NativeError::Status {
            operation,
            code: status as i32,
        });
    }

    value_from_native_handle_with_provenance(out, GerbilValueProvenance::SchemeObjectExport)
        .map_or_else(
            || {
                NativeResult::err(NativeError::Status {
                    operation,
                    code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
                })
            },
            NativeResult::ok,
        )
}

fn value_from_native_handle_with_provenance<'runtime>(
    raw: gerbil_scheme_sys::GerbilValueHandle,
    provenance: GerbilValueProvenance,
) -> Option<GerbilValue<'runtime>> {
    NonZeroUsize::new(raw).map(|raw| GerbilValue {
        raw,
        provenance,
        _runtime: PhantomData,
    })
}

impl GerbilValue<'_> {
    /// Wrap a raw runtime-borrowed value handle, rejecting zero handles.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::Status`] with `NullPointer` when `raw` is zero.
    pub fn from_raw(raw: gerbil_scheme_sys::GerbilValueHandle) -> Result<Self, NativeError> {
        let raw = NonZeroUsize::new(raw).ok_or(NativeError::Status {
            operation: "GerbilValue::from_raw",
            code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
        })?;

        Ok(Self {
            raw,
            provenance: GerbilValueProvenance::UntrustedRaw,
            _runtime: PhantomData,
        })
    }

    /// Return the raw borrowed value handle.
    #[must_use]
    pub fn as_raw(self) -> gerbil_scheme_sys::GerbilValueHandle {
        self.raw.get()
    }

    /// Return the provenance attached to this borrowed value handle.
    #[must_use]
    pub const fn provenance(self) -> GerbilValueProvenance {
        self.provenance
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

impl GerbilI64CallbackAbi<'_> {
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

    /// Returns an opaque sentinel value handle through the initialized runtime path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn runtime_sentinel_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut out = 0;
        // SAFETY: self proves runtime/module lifetime and `out` is a valid
        // output slot for one opaque runtime-borrowed value handle.
        let status = unsafe { gerbil_scheme_rust_runtime_sentinel_value(&raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "GerbilRuntime::runtime_sentinel_value",
                code: status as i32,
            });
        }
        value_from_native_handle_with_provenance(out, GerbilValueProvenance::RuntimeSentinel).ok_or(
            NativeError::Status {
                operation: "GerbilRuntime::runtime_sentinel_value",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            },
        )
    }

    /// Returns the Scheme null object through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_null_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut out = 0;
        // SAFETY: self proves runtime/module lifetime and `out` is a valid
        // output slot for one borrowed Scheme object handle.
        let status = unsafe { gerbil_scheme_rust_fixture_null(&raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "GerbilRuntime::fixture_null_value",
                code: status as i32,
            });
        }
        value_from_native_handle_with_provenance(out, GerbilValueProvenance::SchemeObjectExport)
            .ok_or(NativeError::Status {
                operation: "GerbilRuntime::fixture_null_value",
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            })
    }

    /// Returns a Scheme pair fixture through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_pair_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "GerbilRuntime::fixture_pair_value",
            gerbil_scheme_rust_fixture_pair,
        )
    }

    /// Returns a proper Scheme list fixture through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_proper_list_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "GerbilRuntime::fixture_proper_list_value",
            gerbil_scheme_rust_fixture_proper_list,
        )
    }

    /// Returns an improper Scheme list fixture through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_improper_list_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "GerbilRuntime::fixture_improper_list_value",
            gerbil_scheme_rust_fixture_improper_list,
        )
    }

    /// Exports a Scheme true fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_true_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_true",
            gerbil_scheme_rust_fixture_true,
        )
    }

    /// Exports a Scheme false fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_false_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_false",
            gerbil_scheme_rust_fixture_false,
        )
    }

    /// Exports a Scheme fixnum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_fixnum_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_fixnum",
            gerbil_scheme_rust_fixture_fixnum,
        )
    }

    fn checked_scheme_object_fixture(
        &self,
        operation: &'static str,
        fixture: unsafe extern "C" fn(
            *mut gerbil_scheme_sys::GerbilValueHandle,
        ) -> gerbil_scheme_sys::GerbilStatus,
    ) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut out = 0;
        // SAFETY: self proves runtime/module lifetime and `out` is a valid
        // output slot for one borrowed Scheme object handle.
        let status = unsafe { fixture(&raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation,
                code: status as i32,
            });
        }
        value_from_native_handle_with_provenance(out, GerbilValueProvenance::SchemeObjectExport)
            .ok_or(NativeError::Status {
                operation,
                code: gerbil_scheme_sys::GerbilStatus::NullPointer as i32,
            })
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
