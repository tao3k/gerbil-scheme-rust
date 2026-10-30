//! Loading may be undone only before the image attempts native setup.
use super::InstanceError;
use std::{ffi::c_void, path::PathBuf, sync::Arc};
#[derive(Debug)]
pub(super) struct Preflight {
    mapping: Arc<super::mapping::Mapping>,
    path: PathBuf,
    uninitialized: bool,
}
impl Preflight {
    pub(super) fn new(mapping: Arc<super::mapping::Mapping>, path: PathBuf) -> Self {
        Self {
            mapping,
            path,
            uninitialized: true,
        }
    }
    pub(super) fn setup_attempted(&mut self) {
        self.uninitialized = false;
        self.mapping.initialized();
    }
}
impl Drop for Preflight {
    fn drop(&mut self) {
        if !self.uninitialized {
            return;
        }
        // SAFETY: constructors may not initialize a VM; native setup was never
        // attempted. No Scheme callback or native process hook references code.
        if self.mapping.reject_uninitialized() {
            let _ = std::fs::remove_file(&self.path);
        } else {
            crate::instance_backend::poison_images();
        }
    }
}

unsafe extern "C" {
    fn dlsym(handle: *mut std::ffi::c_void, name: *const std::ffi::c_char)
    -> *mut std::ffi::c_void;
}
pub(super) unsafe fn symbol(
    handle: *mut c_void,
    name: &std::ffi::CStr,
) -> Result<*mut c_void, InstanceError> {
    // SAFETY: retained image handle and NUL-terminated symbol name.
    let address = unsafe { dlsym(handle, name.as_ptr()) };
    if address.is_null() {
        Err(InstanceError::Loader(format!("missing export {name:?}")))
    } else {
        Ok(address)
    }
}
