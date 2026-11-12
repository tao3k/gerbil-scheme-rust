//! Current Mach/proc/dyld metrics. Sampling releases Mach port rights and arrays.
use std::{
    ffi::{CString, c_void},
    marker::PhantomData,
    rc::Rc,
};
unsafe extern "C" {
    fn dlopen(path: *const i8, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const i8) -> *mut c_void;
}
#[derive(Debug)]
pub(super) struct Observer {
    sample: unsafe extern "C" fn(*mut u64) -> i32,
    relief: unsafe extern "C" fn() -> u64,
    tls: unsafe extern "C" fn() -> i32,
    regions: unsafe extern "C" fn(*const i8, *mut u64) -> i32,
    _owner: PhantomData<Rc<()>>,
}
impl Observer {
    pub(super) fn new() -> Self {
        let path = CString::new(gerbil_scheme_qualification::owner_instance_image()).unwrap();
        // SAFETY: trusted qualification image; only a process-resource function
        // runs in this original image. No Scheme VM is initialized by sampling.
        unsafe {
            let handle = dlopen(path.as_ptr(), 2 | 4);
            assert!(!handle.is_null());
            let address = dlsym(handle, c"gerbil_owner_process_resources".as_ptr());
            assert!(!address.is_null());
            Self {
                sample: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut u64) -> i32>(
                    address,
                ),
                relief: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> u64>(dlsym(
                    handle,
                    c"gerbil_owner_allocator_relief".as_ptr(),
                )),
                tls: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(dlsym(
                    handle,
                    c"gerbil_owner_tls_available".as_ptr(),
                )),
                regions: std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(*const i8, *mut u64) -> i32,
                >(dlsym(handle, c"gerbil_owner_memory_regions".as_ptr())),
                _owner: PhantomData,
            }
        }
    }
    pub(super) fn retain_uninitialized(path: &std::path::Path) {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: same build-owned image contract, constructors never initialize
        // a VM. This diagnostic intentionally retains the reference as a control.
        let handle = unsafe { dlopen(path.as_ptr(), 2 | 4) };
        assert!(!handle.is_null());
    }
    pub(super) fn tls(&self, phase: &str) -> i32 {
        // SAFETY: test runs at a quiescent boundary and releases every diagnostic key.
        let free = unsafe { (self.tls)() };
        assert!(free >= 0);
        eprintln!("TLS-CAPACITY phase={phase} available={free}");
        free
    }
    pub(super) fn regions(&self, phase: &str) -> [u64; 12] {
        let prefix = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("gerbil-owners-{}-", std::process::id()));
        let prefix = CString::new(prefix.as_os_str().as_encoded_bytes()).unwrap();
        let mut values = [0; 12];
        // SAFETY: fixed twelve-u64 output and live NUL-terminated directory prefix.
        assert_eq!(
            unsafe { (self.regions)(prefix.as_ptr(), values.as_mut_ptr()) },
            0
        );
        eprintln!(
            "REGIONS phase={phase} malloc-virtual={} malloc-resident={} malloc-dirtied={} malloc-reusable={} image-text-virtual={} image-text-incore={} image-data-virtual={} image-data-incore={} image-data-modified={} owner-images={} image-virtual={} image-incore={}",
            values[0],
            values[1],
            values[2],
            values[3],
            values[4],
            values[5],
            values[6],
            values[7],
            values[8],
            values[9],
            values[10],
            values[11]
        );
        values
    }
    pub(super) fn relieve(&self) -> u64 {
        // SAFETY: test-only allocator pressure after all native owners joined;
        // this cannot release live allocations and does not alter Scheme policy.
        unsafe { (self.relief)() }
    }
    pub(super) fn read(&self) -> [u64; 8] {
        let mut values = [0; 8];
        // SAFETY: exactly eight writable u64 metrics; no Scheme state access.
        assert_eq!(unsafe { (self.sample)(values.as_mut_ptr()) }, 0);
        values
    }
    pub(super) fn settled(&self, phase: &str, epoch: usize, threads: u64) -> [u64; 8] {
        let start = std::time::Instant::now();
        let first = self.read()[3];
        while self.read()[3] > threads {
            assert!(
                start.elapsed() < super::GATE,
                "Mach owner thread did not retire"
            );
            std::thread::yield_now();
        }
        if first > threads {
            eprintln!(
                "RESOURCE-QUIESCENCE phase={phase} epoch={epoch} initial-threads={first} target={threads} elapsed-ns={}",
                start.elapsed().as_nanos()
            );
        }
        self.emit(phase, epoch, &[])
    }
    pub(super) fn emit(&self, phase: &str, epoch: usize, usage: &[[f64; 4]]) -> [u64; 8] {
        let values = self.read();
        eprintln!(
            "RESOURCE-SNAPSHOT phase={phase} epoch={epoch} resident={} footprint={} fds={} threads={} images={} malloc-live={} malloc-reserved={} malloc-blocks={} gc={:?} capacity={:?} live={:?} roots={:?}",
            values[0],
            values[1],
            values[2],
            values[3],
            values[4],
            values[5],
            values[6],
            values[7],
            usage.iter().map(|value| value[0]).collect::<Vec<_>>(),
            usage.iter().map(|value| value[1]).collect::<Vec<_>>(),
            usage.iter().map(|value| value[2]).collect::<Vec<_>>(),
            usage.iter().map(|value| value[3]).collect::<Vec<_>>()
        );
        values
    }
}

/// Optional, post-acceptance diagnosis; never runs inside the native watchdog.
pub(super) fn capture_graph() {
    let directory =
        std::env::temp_dir().join(format!("gerbil-native-final-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    eprintln!(
        "NATIVE-GATES-COMPLETE allocation-graph={}",
        directory.display()
    );
    let graph = directory.join("final.memgraph");
    let output = std::process::Command::new("/usr/bin/leaks")
        .arg("--noContent")
        .arg(format!("--outputGraph={}", graph.display()))
        .arg(std::process::id().to_string())
        .output()
        .unwrap();
    std::fs::write(directory.join("stdout.log"), &output.stdout).unwrap();
    std::fs::write(directory.join("stderr.log"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "allocation graph failed: {:?}",
        output.status
    );
}
