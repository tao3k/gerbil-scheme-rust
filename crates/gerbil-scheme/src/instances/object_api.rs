//! Optional full object ABI. Resolve within the private image before setup.
use super::preflight::symbol;
use std::ffi::c_void;
#[derive(Debug)]
pub(super) struct ObjectApi {
    pub kind: unsafe extern "C" fn(i64, i64) -> i32,
    pub integer: unsafe extern "C" fn(*const u8, u64, i32) -> i64,
    pub integer_size: unsafe extern "C" fn(i64) -> i64,
    pub integer_copy: unsafe extern "C" fn(i64, *mut u8, u64) -> i32,
    pub length: unsafe extern "C" fn(i64, i32) -> i64,
    pub snapshot: unsafe extern "C" fn(i64, i32, u64) -> i64,
    pub table: unsafe extern "C" fn(*const i64, u64, i32) -> i64,
    pub number_create: Option<unsafe extern "C" fn(*const i64, u64, i32) -> i64>,
    pub composite: unsafe extern "C" fn(i64, i64, i32, *mut u64) -> i64,
    pub identical: unsafe extern "C" fn(i64, i64, i64, i64) -> i32,
}
impl ObjectApi {
    pub(super) unsafe fn load(handle: *mut c_void) -> Option<Self> {
        // SAFETY: optional names reuse the existing typed strong-root ABI exactly.
        unsafe {
            Some(Self {
                kind: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64, i64) -> i32>(
                    symbol(handle, c"gerbil_scheme_rust_object_kind").ok()?,
                ),
                integer: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*const u8, u64, i32) -> i64,
                >(
                    symbol(handle, c"gerbil_scheme_rust_bigint_create_raw").ok()?
                ),
                integer_size: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_bigint_size_raw").ok()?,
                ),
                integer_copy: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, *mut u8, u64) -> i32,
                >(
                    symbol(handle, c"gerbil_scheme_rust_bigint_copy_raw").ok()?
                ),
                length: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64, i32) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_collection_length_raw").ok()?,
                ),
                snapshot: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i32, u64) -> i64,
                >(
                    symbol(handle, c"gerbil_scheme_rust_collection_snapshot_raw").ok()?
                ),
                table: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*const i64, u64, i32) -> i64,
                >(symbol(handle, c"gerbil_scheme_rust_table_create").ok()?),
                number_create: symbol(handle, c"gerbil_scheme_rust_composite_create")
                    .ok()
                    .map(|address| {
                        std::mem::transmute::<
                            *mut c_void,
                            unsafe extern "C" fn(*const i64, u64, i32) -> i64,
                        >(address)
                    }),
                composite: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i64, i32, *mut u64) -> i64,
                >(
                    symbol(handle, c"gerbil_scheme_rust_composite_snapshot").ok()?
                ),
                identical: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(i64, i64, i64, i64) -> i32,
                >(
                    symbol(handle, c"gerbil_scheme_rust_object_identical").ok()?
                ),
            })
        }
    }
}
