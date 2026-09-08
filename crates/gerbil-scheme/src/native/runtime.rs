// SPDX-License-Identifier: LGPL-2.1-or-later OR Apache-2.0

//! Gerbil runtime entry points, callbacks, and safe native call boundaries.

use super::integer::{
    resolved_signed_encoding_width, resolved_unsigned_encoding_width,
    value_from_native_handle_with_provenance,
};
use super::rooted::{rooted_exact_integer, rooted_integer_bytevector};
use super::types::RootedSchemeOwner;
use super::{
    BytestringDelimiter, GerbilRuntime, GerbilRuntimeReceipt, GerbilValue, GerbilValueProvenance,
    IntegerEncoding, LinkedGerbilProgram, NativeError, RootedSchemeBytevector,
    RootedSchemeExactInteger,
};
use gerbil_scheme_sys::{
    GERBIL_SCHEME_RUST_ABI_VERSION, gerbil_scheme_rust_abi_version, gerbil_scheme_rust_add_i64,
    gerbil_scheme_rust_fixture_char_ascii, gerbil_scheme_rust_fixture_char_bmp,
    gerbil_scheme_rust_fixture_char_non_bmp,
    gerbil_scheme_rust_fixture_exact_integer_large_negative,
    gerbil_scheme_rust_fixture_exact_integer_large_positive, gerbil_scheme_rust_fixture_false,
    gerbil_scheme_rust_fixture_fixnum, gerbil_scheme_rust_fixture_flonum_finite,
    gerbil_scheme_rust_fixture_flonum_nan, gerbil_scheme_rust_fixture_flonum_neg_inf,
    gerbil_scheme_rust_fixture_flonum_neg_zero, gerbil_scheme_rust_fixture_flonum_pos_inf,
    gerbil_scheme_rust_fixture_improper_list, gerbil_scheme_rust_fixture_null,
    gerbil_scheme_rust_fixture_pair, gerbil_scheme_rust_fixture_proper_list,
    gerbil_scheme_rust_fixture_true, gerbil_scheme_rust_identity_i64,
    gerbil_scheme_rust_runtime_cleanup, gerbil_scheme_rust_runtime_init,
    gerbil_scheme_rust_runtime_sentinel_value,
};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;

static RUNTIME_LIFECYCLE: Mutex<()> = Mutex::new(());
const RUNTIME_NEVER_INITIALIZED: u8 = 0;
const RUNTIME_RUNNING: u8 = 1;
const RUNTIME_FINALIZED: u8 = 2;

static RUNTIME_STATE: AtomicU8 = AtomicU8::new(RUNTIME_NEVER_INITIALIZED);

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
        Self::initialize_with_linker(None)
    }

    /// Initialize one statically linked downstream program with the shared
    /// native lifecycle and GC owner.
    pub fn initialize_program(program: LinkedGerbilProgram) -> Result<Self, NativeError> {
        Self::initialize_with_linker(Some(program.linker))
    }

    fn initialize_with_linker(
        linker: Option<gerbil_scheme_sys::GerbilProgramLinker>,
    ) -> Result<Self, NativeError> {
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
        let status = unsafe {
            match linker {
                Some(linker) => {
                    gerbil_scheme_sys::gerbil_scheme_rust_runtime_init_program(Some(linker))
                }
                None => gerbil_scheme_rust_runtime_init(),
            }
        };
        if status != 0 {
            RUNTIME_STATE.store(RUNTIME_FINALIZED, Ordering::Release);
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
            RUNTIME_STATE.store(RUNTIME_FINALIZED, Ordering::Release);
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

    /// Returns a stable receipt for the initialized runtime binding surface.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] if called outside the initializing
    /// thread.
    pub fn receipt(&self) -> Result<GerbilRuntimeReceipt, NativeError> {
        self.check_thread()?;
        Ok(GerbilRuntimeReceipt {
            abi_id: gerbil_scheme_sys::GERBIL_SCHEME_RUST_ABI_ID,
            abi_version: GERBIL_SCHEME_RUST_ABI_VERSION,
            header_path: gerbil_scheme_sys::GERBIL_SCHEME_RUST_HEADER_PATH,
            native_module_path: GerbilRuntimeReceipt::NATIVE_MODULE_PATH,
        })
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

    /// Returns the Scheme void object through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_void_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut out = 0;
        // SAFETY: self proves runtime/module lifetime and `out` is a valid
        // output slot for one borrowed Scheme object handle.
        let status = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_fixture_void(&raw mut out) };
        if status != gerbil_scheme_sys::GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "GerbilRuntime::fixture_void_value",
                code: status as i32,
            });
        }
        value_from_native_handle_with_provenance(out, GerbilValueProvenance::SchemeObjectExport)
            .ok_or(NativeError::Status {
                operation: "GerbilRuntime::fixture_void_value",
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

    /// Exports an ASCII Scheme character fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_char_ascii_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_char_ascii",
            gerbil_scheme_rust_fixture_char_ascii,
        )
    }

    /// Exports a BMP Scheme character fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_char_bmp_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_char_bmp",
            gerbil_scheme_rust_fixture_char_bmp,
        )
    }

    /// Exports a non-BMP Scheme character fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_char_non_bmp_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_char_non_bmp",
            gerbil_scheme_rust_fixture_char_non_bmp,
        )
    }

    /// Exports a finite Scheme flonum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_flonum_finite_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_flonum_finite",
            gerbil_scheme_rust_fixture_flonum_finite,
        )
    }

    /// Exports a NaN Scheme flonum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_flonum_nan_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_flonum_nan",
            gerbil_scheme_rust_fixture_flonum_nan,
        )
    }

    /// Exports a positive infinity Scheme flonum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_flonum_pos_inf_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_flonum_pos_inf",
            gerbil_scheme_rust_fixture_flonum_pos_inf,
        )
    }

    /// Exports a negative infinity Scheme flonum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_flonum_neg_inf_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_flonum_neg_inf",
            gerbil_scheme_rust_fixture_flonum_neg_inf,
        )
    }

    /// Exports a negative-zero Scheme flonum fixture through the initialized runtime.
    ///
    /// # Errors
    ///
    /// Returns a native error if the fixture export fails.
    pub fn fixture_flonum_neg_zero_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "gerbil_scheme_rust_fixture_flonum_neg_zero",
            gerbil_scheme_rust_fixture_flonum_neg_zero,
        )
    }

    /// Returns a Scheme bytevector fixture through the initialized Gerbil export path.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] when called from a non-owner thread,
    /// or [`NativeError::Status`] when the native export reports an error or
    /// returns a zero handle.
    pub fn fixture_bytevector_value(&self) -> Result<GerbilValue<'_>, NativeError> {
        self.checked_scheme_object_fixture(
            "GerbilRuntime::fixture_bytevector_value",
            gerbil_scheme_sys::gerbil_scheme_rust_fixture_bytevector,
        )
    }

    /// Encode an unsigned integer as a newly rooted Scheme bytevector.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error, or [`NativeError::UnsignedIntegerWidth`]
    /// when a non-truncating fixed width cannot represent `value`.
    pub fn uint_to_bytevector(
        &self,
        value: u64,
        encoding: IntegerEncoding,
    ) -> Result<RootedSchemeBytevector<'_>, NativeError> {
        self.check_thread()?;
        let size = resolved_unsigned_encoding_width(value, encoding)?;
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_uint_to_bytevector_root(
                value,
                encoding.byte_order().abi_code(),
                size,
                &raw mut root,
            )
        };
        rooted_integer_bytevector(status, root, "gerbil_scheme_rust_uint_to_bytevector_root")
    }

    /// Encode a signed integer as a newly rooted two's-complement bytevector.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error, or [`NativeError::SignedIntegerWidth`]
    /// when a non-truncating fixed width cannot represent `value`.
    pub fn sint_to_bytevector(
        &self,
        value: i64,
        encoding: IntegerEncoding,
    ) -> Result<RootedSchemeBytevector<'_>, NativeError> {
        self.check_thread()?;
        let size = resolved_signed_encoding_width(value, encoding)?;
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_sint_to_bytevector_root(
                value,
                encoding.byte_order().abi_code(),
                size,
                &raw mut root,
            )
        };
        rooted_integer_bytevector(status, root, "gerbil_scheme_rust_sint_to_bytevector_root")
    }

    /// Parse an ASCII hexadecimal bytestring through Gerbil's AOT converter.
    ///
    /// The returned bytevector is rooted in the Gerbil module and releases its
    /// root automatically on drop.
    ///
    /// # Errors
    ///
    /// Returns [`NativeError::WrongThread`] outside the runtime owner thread,
    /// or [`NativeError::Status`] when the bytestring or delimiter does not
    /// satisfy Gerbil's conversion contract.
    pub fn bytevector_from_bytestring(
        &self,
        bytestring: &str,
        delimiter: BytestringDelimiter,
    ) -> Result<RootedSchemeBytevector<'_>, NativeError> {
        self.check_thread()?;
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_bytestring_to_bytevector_root(
                gerbil_scheme_sys::GerbilBorrowedUtf8::from(bytestring),
                delimiter.abi_code(),
                &raw mut root,
            )
        };
        RootedSchemeOwner::new(
            status,
            root,
            "gerbil_scheme_rust_bytestring_to_bytevector_root",
        )
        .map(|owner| RootedSchemeBytevector { owner })
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
    /// Export a positive exact-integer fixture larger than `u64::MAX`.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error when the runtime cannot export the fixture.
    pub fn fixture_exact_integer_large_positive_value(
        &self,
    ) -> Result<GerbilValue<'_>, NativeError> {
        self.scheme_object_fixture(
            "GerbilRuntime::fixture_exact_integer_large_positive_value",
            gerbil_scheme_rust_fixture_exact_integer_large_positive,
        )
    }

    /// Export a negative exact-integer fixture smaller than `i64::MIN`.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error when the runtime cannot export the fixture.
    pub fn fixture_exact_integer_large_negative_value(
        &self,
    ) -> Result<GerbilValue<'_>, NativeError> {
        self.scheme_object_fixture(
            "GerbilRuntime::fixture_exact_integer_large_negative_value",
            gerbil_scheme_rust_fixture_exact_integer_large_negative,
        )
    }

    /// Construct an owned, rooted Scheme exact integer from an `i64`.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error if the native root cannot be created.
    pub fn exact_integer_from_i64(
        &self,
        value: i64,
    ) -> Result<RootedSchemeExactInteger<'_>, NativeError> {
        self.check_thread()?;
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_i64_to_exact_integer_root(value, &raw mut root)
        };
        rooted_exact_integer(status, root, "gerbil_scheme_rust_i64_to_exact_integer_root")
    }

    /// Construct an owned, rooted Scheme exact integer from a `u64`.
    ///
    /// Values larger than `i64::MAX` remain exact Scheme bignums.
    ///
    /// # Errors
    ///
    /// Returns a thread/status error if the native root cannot be created.
    pub fn exact_integer_from_u64(
        &self,
        value: u64,
    ) -> Result<RootedSchemeExactInteger<'_>, NativeError> {
        self.check_thread()?;
        let mut root = gerbil_scheme_sys::GerbilRootId(0);
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_u64_to_exact_integer_root(value, &raw mut root)
        };
        rooted_exact_integer(status, root, "gerbil_scheme_rust_u64_to_exact_integer_root")
    }

    fn scheme_object_fixture(
        &self,
        operation: &'static str,
        fixture: unsafe extern "C" fn(
            *mut gerbil_scheme_sys::GerbilValueHandle,
        ) -> gerbil_scheme_sys::GerbilStatus,
    ) -> Result<GerbilValue<'_>, NativeError> {
        self.check_thread()?;
        let mut out = 0;
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

    /// # Errors
    ///
    /// Returns [`NativeError`] when the native runtime rejects the comparison
    /// or returns an invalid ordering value.
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

    pub(super) fn check_thread(&self) -> Result<(), NativeError> {
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
