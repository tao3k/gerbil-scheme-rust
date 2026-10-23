//! The existing native strong-root/type ABI, resolved within one runtime image.
use super::{InstanceError, preflight::symbol};
use gerbil_scheme_sys::GerbilScalarProjection;
use std::ffi::c_void;

type NumericBatchCreate = unsafe extern "C" fn(i32, *const *const u8, *const u64, u64, u64) -> i64;
#[cfg(feature = "tokio-dispatch")]
type NumericServiceCreate =
    unsafe extern "C" fn(i32, *const *const u8, *const u64, u64, u64, *const i64, u64) -> i64;
#[cfg(feature = "tokio-dispatch")]
type NumericServiceEnter =
    unsafe extern "C" fn(i32, *const *const u8, *const u64, u64, u64, *const i64, u64, i64) -> i64;
#[derive(Debug)]
pub(super) struct NativeApi {
    pub setup_diagnostic: Option<unsafe extern "C" fn(*mut i64) -> i32>,
    pub call_error: Option<unsafe extern "C" fn() -> i64>,
    pub objects: Option<super::object_api::ObjectApi>,
    pub numeric_create: unsafe extern "C" fn(i32, *const u8, u64) -> i64,
    pub numeric_batch_create: Option<NumericBatchCreate>,
    #[cfg(feature = "tokio-dispatch")]
    pub numeric_service_create: Option<NumericServiceCreate>,
    #[cfg(feature = "tokio-dispatch")]
    pub numeric_service_enter: Option<NumericServiceEnter>,
    #[cfg(feature = "tokio-dispatch")]
    pub numeric_batch_transfer: Option<super::numeric_snapshot::Transfer>,
    pub numeric_size: unsafe extern "C" fn(i64, i64, i32) -> i64,
    pub numeric_copy: unsafe extern "C" fn(i64, i64, i32, *mut u8, u64, u64) -> i64,
    pub scalar_create: unsafe extern "C" fn(i32, i64, f64) -> i64,
    pub scalar_read: unsafe extern "C" fn(i64, i64, *mut GerbilScalarProjection) -> i32,
    pub utf8_string: unsafe extern "C" fn(i64) -> i64,
    pub string_utf8: unsafe extern "C" fn(i64) -> i64,
    pub collection: unsafe extern "C" fn(*const i64, u64, i32) -> i64,
    pub element: unsafe extern "C" fn(i64, i32, u64) -> i64,
    #[cfg(feature = "tokio-dispatch")]
    pub result_slot_take: Option<unsafe extern "C" fn(i64, u64) -> i64>,
    pub force_gc: unsafe extern "C" fn() -> i32,
    pub diagnostics: Option<unsafe extern "C" fn(*mut f64) -> i32>,
    pub usage: unsafe extern "C" fn(*mut f64) -> i32,
}
impl NativeApi {
    pub(super) fn setup_failure(&self) -> Option<Box<[i64; 4]>> {
        let read = self.setup_diagnostic?;
        let mut values = [0; 4];
        // SAFETY: trusted optional export, resolved before setup; the private
        // image remains loaded, and this is called on its rejected setup owner.
        (unsafe { read(values.as_mut_ptr()) } == 0).then(|| Box::new(values))
    }

    pub(super) unsafe fn load(handle: *mut c_void) -> Result<Self, InstanceError> {
        // SAFETY: the trusted image exports the existing typed bridge ABI; all
        // symbols are checked before any runtime initialization.
        unsafe {
            Ok(Self {
                setup_diagnostic: symbol(handle, c"gerbil_scheme_instance_setup_diagnostic")
                    .ok()
                    .map(|address| {
                        std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut i64) -> i32>(
                            address,
                        )
                    }),
                call_error: symbol(handle, c"gerbil_scheme_rust_take_call_error_raw")
                    .ok()
                    .map(|address| {
                        std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i64>(address)
                    }),
                objects: super::object_api::ObjectApi::load(handle),
                diagnostics: symbol(handle, c"gerbil_scheme_instance_diagnostics")
                    .ok()
                    .map(|address| {
                        std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut f64) -> i32>(
                            address,
                        )
                    }),
                #[cfg(feature = "tokio-dispatch")]
                result_slot_take: result_slot(handle),
                numeric_batch_create: symbol(handle, c"gerbil_scheme_rust_numeric_batch_create")
                    .ok()
                    .map(|address| std::mem::transmute::<*mut c_void, NumericBatchCreate>(address)),
                #[cfg(feature = "tokio-dispatch")]
                numeric_service_create: numeric_service(handle),
                #[cfg(feature = "tokio-dispatch")]
                numeric_service_enter: numeric_entry(handle),
                #[cfg(feature = "tokio-dispatch")]
                numeric_batch_transfer: super::numeric_snapshot::Transfer::load(handle),
                numeric_create: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i32, *const u8, u64) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_numeric_create",
                )?),
                numeric_size: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i64, i32) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_numeric_size",
                )?),
                numeric_copy: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i64, i32, *mut u8, u64, u64) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_numeric_copy",
                )?),
                scalar_create: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i32, i64, f64) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_scalar_object",
                )?),
                scalar_read: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i64, *mut GerbilScalarProjection) -> i32,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_object_scalar_indexed",
                )?),
                utf8_string: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_root_utf8_to_string_raw")?,
                ),
                string_utf8: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_root_string_to_utf8_raw")?,
                ),
                collection: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*const i64, u64, i32) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_collection_create_raw",
                )?),
                element: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i32, u64) -> i64,
                >(symbol(
                    handle,
                    c"gerbil_scheme_rust_collection_element_raw",
                )?),
                force_gc: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(
                    symbol(handle, c"gerbil_scheme_rust_force_gc_raw")?,
                ),
                usage: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut f64) -> i32>(
                    symbol(handle, c"gerbil_scheme_instance_usage")?,
                ),
            })
        }
    }
}

#[cfg(feature = "tokio-dispatch")]
unsafe fn numeric_service(handle: *mut c_void) -> Option<NumericServiceCreate> {
    // SAFETY: same audited image-local symbol contract as NativeApi::load.
    unsafe {
        symbol(handle, c"gerbil_scheme_rust_numeric_service_create")
            .ok()
            .map(|address| std::mem::transmute::<*mut c_void, NumericServiceCreate>(address))
    }
}

#[cfg(feature = "tokio-dispatch")]
unsafe fn numeric_entry(handle: *mut c_void) -> Option<NumericServiceEnter> {
    // SAFETY: optional, audited image-local typed entry ABI, resolved before setup.
    unsafe {
        symbol(handle, c"gerbil_scheme_rust_numeric_service_enter")
            .ok()
            .map(|address| std::mem::transmute::<*mut c_void, NumericServiceEnter>(address))
    }
}

#[cfg(feature = "tokio-dispatch")]
unsafe fn result_slot(handle: *mut c_void) -> Option<unsafe extern "C" fn(i64, u64) -> i64> {
    // SAFETY: optional audited image-local slot-transfer ABI, before native setup.
    unsafe {
        symbol(handle, c"gerbil_scheme_rust_result_slot_take")
            .ok()
            .map(|address| {
                std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64, u64) -> i64>(address)
            })
    }
}
