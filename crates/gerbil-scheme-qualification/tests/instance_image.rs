// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Functional qualification of the canonical owner image, without a benchmark.
#![cfg(target_os = "macos")]
use std::ffi::{CStr, CString};

fn available_tls_keys() -> usize {
    // SAFETY: POSIX exposes this process's finite thread-key capacity.
    let limit = unsafe { libc::sysconf(libc::_SC_THREAD_KEYS_MAX) };
    let limit = usize::try_from(limit).expect("finite Darwin TLS key capacity");
    let mut keys = Vec::new();
    let mut exhausted = false;
    for _ in 0..limit {
        let mut key = 0;
        // SAFETY: fresh output and no destructor; every successful key is retired below.
        let status = unsafe { libc::pthread_key_create(&raw mut key, None) };
        if status != 0 {
            exhausted = status == libc::EAGAIN;
            break;
        }
        keys.push(key);
    }
    for key in &keys {
        // SAFETY: this probe owns these keys and has installed no values.
        assert_eq!(unsafe { libc::pthread_key_delete(*key) }, 0);
    }
    assert!(exhausted, "TLS probe did not reach the platform capacity");
    keys.len()
}

struct Image {
    handle: *mut libc::c_void,
    live: bool,
}

impl Image {
    fn symbol(&self, name: &CStr) -> *mut libc::c_void {
        // SAFETY: this live handle and terminated name belong to the fixture.
        let symbol = unsafe { libc::dlsym(self.handle, name.as_ptr()) };
        assert!(!symbol.is_null(), "missing owner export: {name:?}");
        symbol
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if !self.live && !self.handle.is_null() {
            // SAFETY: no SDK frame or processor is live in this image.
            assert_eq!(unsafe { libc::dlclose(self.handle) }, 0);
        }
    }
}

#[test]
fn canonical_image_sets_up_one_owner_and_cleans_up_before_unload() {
    type Setup = unsafe extern "C" fn(u32) -> i32;
    type Cleanup = unsafe extern "C" fn() -> i32;
    type Processors = unsafe extern "C" fn() -> u32;
    std::thread::spawn(|| ()).join().unwrap();
    let path = CString::new(env!("GERBIL_CANONICAL_OWNER_IMAGE")).unwrap();
    // SAFETY: the normal build entry produced this image and ABI.
    let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    assert!(!handle.is_null(), "canonical owner image failed to load");
    let mut image = Image {
        handle,
        live: false,
    };
    let tls_before = available_tls_keys();
    // SAFETY: the canonical image owns these exact public C signatures.
    let (setup, cleanup, processors) = unsafe {
        (
            std::mem::transmute::<*mut libc::c_void, Setup>(
                image.symbol(c"gerbil_scheme_instance_setup"),
            ),
            std::mem::transmute::<*mut libc::c_void, Cleanup>(
                image.symbol(c"gerbil_scheme_instance_cleanup"),
            ),
            std::mem::transmute::<*mut libc::c_void, Processors>(
                image.symbol(c"gerbil_scheme_rust_current_vm_processors"),
            ),
        )
    };
    // The image outlives the dedicated owner, including its SDK TLS destructors.
    // Keep code resident if startup or cleanup has an unknown outcome.
    image.live = true;
    let (started, population, duplicate, retired, terminal) = std::thread::spawn(move || {
        // SAFETY: this dedicated thread exclusively owns the resident image.
        let started = unsafe { setup(1) };
        if started != 0 {
            return (started, 0, 0, -1, 0);
        }
        // Collect before asserting: mismatches cannot bypass cleanup.
        // SAFETY: the same owner remains live until cleanup returns.
        let (population, duplicate, retired) = unsafe { (processors(), setup(1), cleanup()) };
        let terminal = if retired == 0 {
            // SAFETY: terminal admission refuses before resource acquisition.
            unsafe { setup(1) }
        } else {
            0
        };
        (started, population, duplicate, retired, terminal)
    })
    .join()
    .expect("owner thread panicked; code retained");
    if retired == 0 && terminal == 8 {
        image.live = false;
    }
    let tls_retired = available_tls_keys();
    drop(image);
    assert_eq!(started, 0);
    assert_eq!(retired, 0, "owner cleanup failed; code retained");
    assert_eq!(population, 1);
    assert_eq!(duplicate, 6);
    assert_eq!(terminal, 8);
    assert_eq!(
        tls_retired, tls_before,
        "retained code must not retain TLS keys after owner join"
    );
}
