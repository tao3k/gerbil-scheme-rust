//! Native callback revocation, thread-exit destructors and failed quiescence.
use super::{GATE, Progress, command, config, normal, observe::Observer, observed};
use gerbil_scheme::instances::{InstanceError, InstancePool};
use std::{
    ffi::c_void,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
unsafe extern "C" {
    fn dlopen(path: *const i8, flags: i32) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlsym(handle: *mut c_void, name: *const i8) -> *mut c_void;
}
#[derive(Debug)]
struct Hooks {
    handle: *mut c_void,
    start: unsafe extern "C" fn(*mut u64) -> i32,
    retire: unsafe extern "C" fn() -> i32,
    state: unsafe extern "C" fn() -> i32,
    callback: Option<unsafe extern "C" fn(u64) -> u64>,
}
impl Hooks {
    fn open(path: &Path, flags: i32) -> Self {
        let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: compiler-owned qualification functions; caller owns the handle.
        unsafe {
            let handle = dlopen(path.as_ptr(), flags);
            assert!(!handle.is_null());
            let symbol = |name: &std::ffi::CStr| {
                let address = dlsym(handle, name.as_ptr());
                assert!(!address.is_null());
                address
            };
            Self {
                handle,
                start: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut u64) -> i32>(
                    symbol(c"gerbil_owner_callback_tls_start"),
                ),
                retire: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(symbol(
                    c"gerbil_owner_callback_tls_retire",
                )),
                state: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(symbol(
                    c"gerbil_owner_runtime_state",
                )),
                callback: Some(std::mem::transmute::<
                    *mut c_void,
                    unsafe extern "C" fn(u64) -> u64,
                >(symbol(c"gerbil_owner_callback"))),
            }
        }
    }
    fn unload(mut self, references: usize) {
        assert!(self.callback.is_none(), "foreign callback must be revoked");
        let handle = self.handle;
        self.handle = std::ptr::null_mut();
        for _ in 0..references {
            // SAFETY: diagnostic explicitly balances known references only after
            // owner join, TLS retirement and callback revocation (except negative).
            assert_eq!(unsafe { dlclose(handle) }, 0);
        }
    }
}
impl Drop for Hooks {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: releases only this observer reference, never the backend's.
            assert_eq!(unsafe { dlclose(self.handle) }, 0);
        }
    }
}
pub(super) fn run(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("callback-baseline", 0, &[]);
    let tls = observer.tls("callback-baseline");
    let counter = Box::new(AtomicU64::new(0));
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    let mut hooks = Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let start = hooks.start;
    let address = (&raw const *counter) as usize;
    pool.dispatcher()
        .submit_native_on(0, move |_| {
            // SAFETY: boxed, aligned counter outlives the owner and its TLS destructor.
            assert_eq!(unsafe { start(address as *mut u64) }, 0);
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    observed(progress, "observed owner TLS registration");
    for (operation, expected) in [(1, 23), (2, 9), (3, 11), (4, 31)] {
        assert_eq!(
            command(&pool.dispatcher(), 0, vec![2, operation, 2], progress),
            [expected]
        );
    }
    // SAFETY: live compiler-owned pure C callback; no dead Scheme entry.
    assert_eq!(unsafe { hooks.callback.unwrap()(41) }, 42);
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "TLS must finish before join"
    );
    // SAFETY: plain adapter state; key retirement follows the owner's destructor.
    assert_eq!(unsafe { (hooks.state)() }, 2);
    assert_eq!(unsafe { (hooks.retire)() }, 0);
    // The host still holds a valid code callback after SDK cleanup; revoke it
    // before balancing the backend's deliberately retained mapping reference.
    assert_eq!(unsafe { hooks.callback.unwrap()(41) }, 42);
    hooks.callback = None;
    hooks.unload(2);
    observed(
        progress,
        "observed joined TLS, revoked callback and diagnostic unload",
    );
    let after = observer.settled("callback-unloaded", 0, baseline[3]);
    assert_eq!(after[4], baseline[4], "qualified copy must actually unmap");
    assert_eq!(observer.tls("callback-unloaded"), tls);
    negative(progress);
    eprintln!(
        "CALLBACK-RETIREMENT native-scheme=true tls-destructors=1 callback-revoked=true actual-unload=true production-policy=retained"
    );
}
pub(super) fn failure(progress: &Progress) {
    // SAFETY: injected export obeys the rooted ABI and rejects the finalizer.
    let program = unsafe { normal().with_finalizer(c"gerbil_owner_failed_finalizer") };
    let mut pool = InstancePool::start(&program, config(1)).unwrap();
    let hooks = Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let _root = pool
        .dispatcher()
        .submit_native_on(0, |session| {
            let value = session.numeric_vector(&[0_i64, 1])?;
            session.retain(value)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    observed(progress, "observed root retained before rejected finalizer");
    let receipt = pool.shutdown(GATE);
    // SAFETY: static adapter flag, not a finalized VM dereference.
    let state = unsafe { (hooks.state)() };
    eprintln!("FINALIZER-FAILURE state={state} receipt={receipt:?}");
    assert!(!receipt.is_clean());
    assert_eq!(receipt.joined, 1);
    assert_eq!(state, 1, "failed finalizer must not destroy the SDK");
    assert_eq!(receipt.owners[0].status, -71);
    assert!(!receipt.owners[0].native_cleanup_attempted);
    assert_eq!(receipt.owners[0].roots_before_cleanup, 1);
    assert_eq!(receipt.owners[0].native_io_released_bytes, 0);
    assert!(pool.image_directory.join("owner-0.dylib").exists());
    assert!(matches!(
        InstancePool::start(&normal(), config(1)),
        Err(InstanceError::BackendConflict)
    ));
    observed(progress, "observed explicit failed-quiescence quarantine");
}
pub(super) fn early_unload(progress: &Progress) {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: applies only to this deliberate negative-control process.
    assert_eq!(
        unsafe { libc::setrlimit(libc::RLIMIT_CORE, &raw const limit) },
        0
    );
    let directory =
        std::env::temp_dir().join(format!("gerbil-tls-negative-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("early.dylib");
    std::fs::copy(gerbil_scheme_qualification::owner_instance_image(), &path).unwrap();
    let mut hooks = Hooks::open(&path, 2 | 4);
    let start = hooks.start;
    let counter = Box::new(AtomicU64::new(0));
    let address = (&raw const *counter) as usize;
    let (ready, entered) = std::sync::mpsc::channel();
    let (leave, release) = std::sync::mpsc::channel();
    let owner = std::thread::spawn(move || {
        // SAFETY: owner and counter remain alive; only the negative deliberately
        // invalidates the destructor's code mapping before thread exit.
        assert_eq!(unsafe { start(address as *mut u64) }, 0);
        ready.send(()).unwrap();
        release.recv_timeout(GATE).unwrap();
    });
    entered.recv_timeout(GATE).unwrap();
    hooks.callback = None;
    hooks.unload(1);
    observed(
        progress,
        "negative: code unmapped before registered TLS destructor",
    );
    leave.send(()).unwrap();
    owner.join().unwrap();
    eprintln!(
        "TLS-NEGATIVE unexpected-normal-return count={}",
        counter.load(Ordering::SeqCst)
    );
}

pub(super) fn unwind(progress: &Progress) {
    struct PanicReply;
    impl Drop for PanicReply {
        fn drop(&mut self) {
            panic!("injected abandoned reply destructor");
        }
    }
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    let hooks = Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let (ready, entered) = std::sync::mpsc::channel();
    let (leave, release) = std::sync::mpsc::channel();
    let ticket = pool
        .dispatcher()
        .submit_native_on(0, move |session| {
            let value = session.numeric_vector(&[9_i64])?;
            session.retain(value)?;
            ready.send(()).unwrap();
            release.recv_timeout(GATE).unwrap();
            Ok(PanicReply)
        })
        .unwrap();
    entered.recv_timeout(GATE).unwrap();
    drop(ticket);
    leave.send(()).unwrap();
    let receipt = pool.shutdown(GATE);
    // SAFETY: static adapter flag independent of VM TLS/heap.
    let state = unsafe { (hooks.state)() };
    eprintln!("OWNER-UNWIND state={state} receipt={receipt:?}");
    assert_eq!(receipt.panicked, 1);
    assert_eq!(receipt.joined, 1);
    assert!(!receipt.is_clean());
    assert_eq!(state, 1, "unverified unwind must not destroy the SDK");
    assert!(pool.image_directory.join("owner-0.dylib").exists());
    assert!(matches!(
        InstancePool::start(&normal(), config(1)),
        Err(InstanceError::BackendConflict)
    ));
    observed(progress, "observed abandoned-reply unwind quarantine");
}

pub(super) fn roots(progress: &Progress) {
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    let hooks = Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let result = pool
        .dispatcher()
        .submit_native_on(0, |session| {
            // Intentional root-balance fault; safe Rust can forget a guard, so the
            // terminal protocol must not claim permission to destroy this heap.
            std::mem::forget(session.string("unbalanced native root")?);
            Ok(())
        })
        .unwrap()
        .wait(GATE);
    assert!(result.is_err());
    let receipt = pool.shutdown(GATE);
    // SAFETY: static adapter flag, no Scheme entry or finalized heap access.
    let state = unsafe { (hooks.state)() };
    eprintln!("ROOT-QUARANTINE state={state} receipt={receipt:?}");
    assert_eq!(state, 1);
    assert_eq!(receipt.joined, 1);
    assert!(!receipt.is_clean());
    assert_eq!(receipt.owners[0].status, -72);
    assert_eq!(receipt.owners[0].roots_before_cleanup, 1);
    assert!(!receipt.owners[0].native_cleanup_attempted);
    assert_eq!(receipt.owners[0].native_io_released_bytes, 0);
    assert!(pool.image_directory.join("owner-0.dylib").exists());
    observed(progress, "observed unbalanced-root quarantine");
}

fn negative(progress: &Progress) {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--tls-unload-negative")
        .output()
        .unwrap();
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.signal(), Some(libc::SIGSEGV));
    observed(
        progress,
        "observed actual early-unload TLS crash in owned child",
    );
    eprintln!("TLS-NEGATIVE expected-signal=SIGSEGV observed=true scheme-setup=false");
}

/// A returned Rust owner closure does not prove that native TLS has retired.
pub(super) fn blocking_tls(observer: &Observer, progress: &Progress, abandon: bool) {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    let baseline = observer.emit("blocking-tls-baseline", 0, &[]);
    let tls = observer.tls("blocking-tls-baseline");
    let flags = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    let hooks = Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let symbol = |name: &std::ffi::CStr| {
        // SAFETY: lookup only; this observer keeps the qualification image live.
        let address = unsafe { dlsym(hooks.handle, name.as_ptr()) };
        assert!(!address.is_null());
        address
    };
    // SAFETY: compiler-owned qualification exports resolved from a live handle.
    let (start, retire) = unsafe {
        (
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut u64) -> i32>(symbol(
                c"gerbil_owner_blocking_tls_start",
            )),
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(symbol(
                c"gerbil_owner_blocking_tls_retire",
            )),
        )
    };
    let address = flags.as_ptr() as usize;
    pool.dispatcher()
        .submit_native_on(0, move |_| {
            // SAFETY: both Arc owners outlive the actual native thread join.
            assert_eq!(unsafe { start(address as *mut u64) }, 0);
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    let _initial = pool.shutdown(Duration::ZERO);
    let entered = Instant::now();
    while flags[0].load(Ordering::SeqCst) == 0 {
        assert!(entered.elapsed() < GATE, "TLS destructor did not enter");
        std::thread::sleep(Duration::from_millis(1));
    }
    observed(
        progress,
        "observed native TLS destructor holding owner exit",
    );
    let release_flags = Arc::clone(&flags);
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(750));
        release_flags[1].store(1, Ordering::SeqCst);
    });
    let start = Instant::now();
    let pending = pool.shutdown(Duration::from_millis(20));
    let elapsed = start.elapsed();
    let complete = if abandon {
        drop(pool);
        release.join().unwrap();
        None
    } else {
        release.join().unwrap();
        let receipt = pool.shutdown(GATE);
        assert!(receipt.is_clean(), "{receipt:?}");
        Some(receipt)
    };
    eprintln!(
        "BLOCKING-TLS elapsed-us={} pending={pending:?} complete={complete:?}",
        elapsed.as_micros()
    );
    assert_eq!(pending.pending, 1, "native destructor is still live");
    assert_eq!(pending.joined, 0);
    assert!(!pending.is_clean());
    assert!(
        elapsed < Duration::from_millis(250),
        "shutdown exceeded its 20ms wait budget"
    );
    let after = observer.settled("blocking-tls-retired", 0, baseline[3]);
    assert_eq!(after[2], baseline[2], "FDs must return after actual join");
    assert_eq!(
        after[3], baseline[3],
        "temporary join coordinator must exit"
    );
    // SAFETY: actual owner exit (and coordinator exit) is observed above.
    assert_eq!(unsafe { retire() }, 0);
    assert_eq!(unsafe { (hooks.state)() }, 2);
    assert_eq!(observer.tls("blocking-tls-retired"), tls);
    observed(
        progress,
        "observed bounded shutdown and later actual native join",
    );
}
