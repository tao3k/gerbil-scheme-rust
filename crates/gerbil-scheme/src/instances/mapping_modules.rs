//! Paired adapter functions transfer SDK-owned module references at cleanup.
use super::{InstanceError, preflight::symbol};
use std::ffi::c_void;
unsafe extern "C" {
    fn dlclose(handle: *mut c_void) -> i32;
}
#[derive(Clone, Copy, Debug)]
pub(super) struct NativeModules {
    leases: unsafe extern "C" fn(u64),
    pending: unsafe extern "C" fn() -> u64,
    release: unsafe extern "C" fn() -> i32,
}
impl NativeModules {
    pub(super) unsafe fn load(handle: *mut c_void) -> Result<Self, InstanceError> {
        // SAFETY: ABI-v4 preflight resolves every function before SDK setup.
        unsafe {
            let bind: unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void) -> i32) =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_bind_loader")?);
            let api =
                Self {
                    leases: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(u64)>(symbol(
                        handle,
                        c"gerbil_scheme_instance_code_leases",
                    )?),
                    pending: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> u64>(
                        symbol(handle, c"gerbil_scheme_instance_pending_modules")?,
                    ),
                    release: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(
                        symbol(handle, c"gerbil_scheme_instance_release_modules")?,
                    ),
                };
            bind(dlclose);
            Ok(api)
        }
    }
    /// Constant scalar setter; no allocations, SDK entry or callbacks.
    pub(super) fn set_leases(self, count: usize) {
        // SAFETY: the mapping owner holds code until this call returns.
        unsafe { (self.leases)(u64::try_from(count).expect("Darwin lease count")) };
    }
    pub(super) fn pending(self) -> u64 {
        // SAFETY: used only while the owner/drain operation holds the base image.
        unsafe { (self.pending)() }
    }
    pub(super) fn release(self) -> i32 {
        // SAFETY: mapping typestate proved actual join and no live code leases.
        unsafe { (self.release)() }
    }
}
