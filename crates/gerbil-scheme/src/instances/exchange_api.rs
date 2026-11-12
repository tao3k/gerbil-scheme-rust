//! Optional, indivisible native exchange capability, resolved before setup.
use super::preflight::symbol;
use std::ffi::c_void;
type Enter =
    unsafe extern "C" fn(i64, i32, *const u8, u64, *const i64, u64, *const i64, u64) -> i64;
type Harvest = unsafe extern "C" fn(i64, *mut i64, u64, *mut i64) -> i64;
type Copy = unsafe extern "C" fn(i64, u64, i32, *mut u8, u64) -> i64;
#[derive(Debug, Clone, Copy)]
pub(super) struct ExchangeApi {
    pub enter: Enter,
    pub harvest: Harvest,
    pub copy: Copy,
}
impl ExchangeApi {
    pub(super) unsafe fn load(handle: *mut c_void) -> Option<Self> {
        // SAFETY: audited signatures in one trusted image; partial ABIs are rejected.
        unsafe {
            Some(Self {
                enter: std::mem::transmute::<*mut c_void, Enter>(
                    symbol(handle, c"gerbil_scheme_rust_exchange_enter").ok()?,
                ),
                harvest: std::mem::transmute::<*mut c_void, Harvest>(
                    symbol(handle, c"gerbil_scheme_rust_exchange_harvest").ok()?,
                ),
                copy: std::mem::transmute::<*mut c_void, Copy>(
                    symbol(handle, c"gerbil_scheme_rust_result_slot_numeric_copy").ok()?,
                ),
            })
        }
    }
}
