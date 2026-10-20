//! The existing native strong-root/type ABI, resolved within one runtime image.
use super::{InstanceError, preflight::symbol};
use gerbil_scheme_sys::GerbilScalarProjection;
use std::ffi::c_void;

#[derive(Debug)]
pub(super) struct NativeApi {
    pub objects: Option<super::object_api::ObjectApi>,
    pub numeric_create: unsafe extern "C" fn(i32, *const u8, u64) -> i64,
    pub numeric_size: unsafe extern "C" fn(i64, i64, i32) -> i64,
    pub numeric_copy: unsafe extern "C" fn(i64, i64, i32, *mut u8, u64, u64) -> i64,
    pub scalar_create: unsafe extern "C" fn(i32, i64, f64) -> i64,
    pub scalar_read: unsafe extern "C" fn(i64, i64, *mut GerbilScalarProjection) -> i32,
    pub utf8_string: unsafe extern "C" fn(i64) -> i64,
    pub string_utf8: unsafe extern "C" fn(i64) -> i64,
    pub collection: unsafe extern "C" fn(*const i64, u64, i32) -> i64,
    pub element: unsafe extern "C" fn(i64, i32, u64) -> i64,
    pub force_gc: unsafe extern "C" fn() -> i32,
    pub diagnostics: Option<unsafe extern "C" fn(*mut f64) -> i32>,
    pub usage: unsafe extern "C" fn(*mut f64) -> i32,
}
impl NativeApi {
    pub(super) unsafe fn load(handle: *mut c_void) -> Result<Self, InstanceError> {
        // SAFETY: the trusted image exports the existing typed bridge ABI; all
        // symbols are checked before any runtime initialization.
        unsafe {
            Ok(Self {
                objects: super::object_api::ObjectApi::load(handle),
                diagnostics: symbol(handle, c"gerbil_scheme_instance_diagnostics")
                    .ok()
                    .map(|address| {
                        std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut f64) -> i32>(
                            address,
                        )
                    }),
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
