//! Darwin-only diagnostic loader. No production runtime ownership is relaxed.

use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    path::Path,
    rc::Rc,
};

unsafe extern "C" {
    fn dlopen(path: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

pub(super) type Batch = unsafe extern "C" fn(u32, u32, u32, *mut c_void) -> i64;

#[derive(Debug)]
pub(super) struct Image {
    batch: Batch,
    marker: unsafe extern "C" fn() -> u32,
    cleanup: unsafe extern "C" fn() -> i32,
    pub(super) state: usize,
    _owner: PhantomData<Rc<()>>,
}

impl Image {
    pub(super) fn load(path: &Path) -> Self {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).expect("image path");
        eprintln!("INSTANCE-LOAD opening={path:?}");
        // SAFETY: this diagnostic loads only Cargo-generated, private images.
        // RTLD_NOW | RTLD_LOCAL on Darwin; no global symbol publication.
        let handle = unsafe { dlopen(path.as_ptr(), 2 | 4) };
        if handle.is_null() {
            // SAFETY: dlerror returns a borrowed loader diagnostic or null.
            let error = unsafe { dlerror() };
            assert!(!error.is_null(), "loader failed without a diagnostic");
            panic!(
                "private image load: {}",
                unsafe { CStr::from_ptr(error) }.to_string_lossy()
            );
        }
        eprintln!("INSTANCE-LOAD loaded=OK");
        // SAFETY: all five signatures are defined in instance_probe.c and the
        // build's export list. This image remains loaded until process exit.
        unsafe {
            let initialize: unsafe extern "C" fn() -> i32 =
                std::mem::transmute(symbol(handle, c"gerbil_instance_probe_init"));
            let state: unsafe extern "C" fn() -> usize =
                std::mem::transmute(symbol(handle, c"gerbil_instance_probe_state"));
            eprintln!("INSTANCE-INIT calling-official-setup");
            assert_eq!(initialize(), 0, "private runtime initialization");
            eprintln!("INSTANCE-INIT completed=OK");
            Self {
                batch: std::mem::transmute::<*mut c_void, Batch>(symbol(
                    handle,
                    c"gerbil_instance_probe_batch",
                )),
                marker: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> u32>(symbol(
                    handle,
                    c"gerbil_instance_probe_marker",
                )),
                cleanup: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(symbol(
                    handle,
                    c"gerbil_instance_probe_cleanup",
                )),
                state: state(),
                _owner: PhantomData,
            }
        }
    }

    pub(super) fn run(&self, id: u32, jobs: u32, rounds: u32, context: *mut c_void) -> i64 {
        // SAFETY: this !Send/!Sync image stays on its initialization thread.
        // The caller retains the optional callback context through this call.
        unsafe { (self.batch)(id, jobs, rounds, context) }
    }

    pub(super) fn marker(&self) -> u32 {
        // SAFETY: the same live VM owner queries its Scheme global.
        unsafe { (self.marker)() }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: owner-affine cleanup after all exports complete. Do NOT
        // dlclose: process signal hooks may still reference image code. This
        // unresolved unload boundary prevents production executor admission.
        assert_eq!(unsafe { (self.cleanup)() }, 0, "image cleanup");
    }
}

unsafe fn symbol(handle: *mut c_void, name: &CStr) -> *mut c_void {
    // SAFETY: handle is a live generated image and name is NUL-terminated.
    let address = unsafe { dlsym(handle, name.as_ptr()) };
    assert!(!address.is_null(), "missing private export {name:?}");
    address
}
