//! Native allocation failure stays quarantined even with zero Scheme roots.
use super::{GATE, Progress, config, observed, program};
use gerbil_scheme::instances::{ImageMappingState, InstancePool};
use std::{
    ffi::{CString, c_void},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
unsafe extern "C" {
    fn dlopen(path: *const i8, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const i8) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}
pub(super) fn run(progress: &Progress) {
    // SAFETY: compiled qualification image; every escaping pure-C call is leased.
    let descriptor = unsafe {
        program(
            gerbil_scheme_qualification::owner_capture_failure_image(),
            c"gerbil_owner_native_call",
        )
        .with_unload_after_join()
    };
    let mut pool = InstancePool::start(&descriptor, config(1)).unwrap();
    let guard = pool
        .dispatcher()
        .submit_native_on(0, |session| Ok(session.code_lease()))
        .unwrap()
        .wait(GATE)
        .unwrap();
    let path = CString::new(
        pool.image_directory
            .join("owner-0.dylib")
            .as_os_str()
            .as_encoded_bytes(),
    )
    .unwrap();
    // SAFETY: image is live, paired native signature; temporary lookup is balanced.
    let roundtrip = unsafe {
        let handle = dlopen(path.as_ptr(), 2 | 4 | 16);
        assert!(!handle.is_null());
        let symbol = dlsym(handle, c"gerbil_owner_loader_roundtrip".as_ptr());
        assert!(!symbol.is_null());
        let function =
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*const i8) -> i32>(symbol);
        assert_eq!(dlclose(handle), 0);
        function
    };
    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::clone(&stop);
    let foreign_guard = guard.clone();
    let (ready, registered) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        // SAFETY: pure C, no Scheme/SDK entry, explicit guard covers every call.
        assert_eq!(unsafe { roundtrip(path.as_ptr()) }, -1);
        ready.send(()).unwrap();
        while !finished.load(Ordering::Acquire) {
            // The file may be unlinked after SDK cleanup; no extra reference
            // exists on a rejected lookup. Successful lookup hits the fault.
            assert!(matches!(unsafe { roundtrip(path.as_ptr()) }, -1 | -2));
            std::thread::yield_now();
        }
        drop(foreign_guard);
    });
    registered.recv_timeout(GATE).unwrap();
    observed(
        progress,
        "observed native retirement-node allocation failure",
    );
    let receipt = pool.shutdown(GATE);
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    assert!(
        !receipt.is_clean(),
        "zero roots cannot authorize failed capture"
    );
    assert_eq!(receipt.pending, 0);
    assert_eq!(receipt.owners[0].roots_before_cleanup, 0);
    assert_eq!(receipt.owners[0].status, -3);
    assert_eq!(receipt.mappings[0].state, ImageMappingState::Quarantined);
    assert!(receipt.mappings[0].pending_module_closes.unwrap() > 0);
    drop(guard);
    assert_eq!(
        pool.shutdown(GATE).mappings[0].state,
        ImageMappingState::Quarantined
    );
    observed(
        progress,
        "observed zero-root capture failure quarantined after foreign join",
    );
    eprintln!(
        "MODULE-CAPTURE-FAILURE zero-roots=true cleanup=-3 quarantined=true foreign-callbacks=true"
    );
}
