//! Qualified release, necessary code leases and foreign-thread TLS lifetime.
use super::{GATE, Progress, command, config, normal, observe::Observer, observed};
use gerbil_scheme::instances::{ImageMappingState, InstancePool};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};

pub(super) fn release(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("mapping-baseline", 0, &[]);
    for cycle in 0..8 {
        // SAFETY: this compiler-owned program has no escaping foreign references.
        // Its owner-only TLS is retired by actual native join, never by a timer.
        let program = unsafe { normal().with_unload_after_join() };
        let startup = std::time::Instant::now();
        let mut pool = InstancePool::start_observed(&program, config(4), |owner| {
            eprintln!(
                "OWNER-READY cycle={cycle} owner={} processors={} setup-ns={}",
                owner.owner,
                owner.processors,
                owner.setup_time.as_nanos()
            );
            observed(progress, "observed successful native owner initialization");
        })
        .unwrap();
        let (resource, path) = super::leases::owned_file(cycle);
        pool.retain_owner_resource(&resource).unwrap();
        drop(resource);
        eprintln!(
            "POOL-READY cycle={cycle} startup-ns={}",
            startup.elapsed().as_nanos()
        );
        super::contracts::types(&pool, progress);
        for owner in 0..4 {
            for (operation, expected) in [(1, 23), (2, 9), (3, 11), (4, 31)] {
                assert_eq!(
                    command(&pool.dispatcher(), owner, vec![2, operation, 2], progress),
                    [expected]
                );
            }
        }
        let receipt = pool.shutdown(GATE);
        assert!(receipt.is_clean(), "{receipt:?}");
        assert!(
            !path.exists(),
            "host resource must retire before clean join publication"
        );
        assert!(pool.retain_owner_resource(&Arc::new(())).is_err());
        assert_eq!(receipt.mappings.len(), 4);
        assert!(
            receipt
                .mappings
                .iter()
                .all(|r| r.state == ImageMappingState::Released)
        );
        let after = observer.settled("mapping-released", cycle, baseline[3]);
        assert_eq!(
            after[2..5],
            baseline[2..5],
            "actual FD/thread/image population"
        );
        observed(
            progress,
            "observed four images released by library after clean join",
        );
        // The close receipt and pool still own bounded Rust metadata above.
        // Observe destruction separately from SDK cleanup and code retirement.
        drop(receipt);
        drop(pool);
        let dropped = observer.settled("mapping-after-host-drop", cycle, baseline[3]);
        assert_eq!(dropped[2..5], baseline[2..5]);
    }
    eprintln!(
        "MAPPING-RELEASE pools=8 owners=32 actual-unmap=true native-types-gc-continuations=true"
    );
}

pub(super) fn foreign_tls(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("lease-baseline", 0, &[]);
    let tls = observer.tls("lease-baseline");
    // SAFETY: every escaping reference below has a code lease; all native
    // references retire before it is dropped. Only pure C runs after SDK cleanup.
    let program = unsafe { normal().with_unload_after_join() };
    let mut pool = InstancePool::start(&program, config(1)).unwrap();
    let mut hooks =
        super::callbacks::Hooks::open(&pool.image_directory.join("owner-0.dylib"), 2 | 4 | 16);
    let lease = pool
        .dispatcher()
        .submit_native_on(0, |session| Ok(session.code_lease()))
        .unwrap()
        .wait(GATE)
        .unwrap();
    let duplicate = lease.clone();
    let counter = Arc::new(AtomicU64::new(0));
    let address = Arc::as_ptr(&counter) as usize;
    let start = hooks.start;
    let (ready, registered) = mpsc::channel();
    let (finish_thread, exit) = mpsc::channel();
    let foreign = std::thread::spawn(move || {
        // SAFETY: parent owns the counter and both code leases until actual join.
        assert_eq!(unsafe { start(address as *mut u64) }, 0);
        ready.send(()).unwrap();
        exit.recv_timeout(GATE).unwrap();
    });
    registered.recv_timeout(GATE).unwrap();
    let mut callback = hooks.callback.take();
    let retire = hooks.retire;
    let state = hooks.state;
    drop(hooks); // Balance the observer only: leases must now protect real code.
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(
        receipt.mappings[0].state,
        ImageMappingState::RetainedByLeases(2)
    );
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "foreign thread is still live"
    );
    // SAFETY: pure native callback is protected by its explicit code leases.
    assert_eq!(unsafe { callback.take().unwrap()(41) }, 42);
    assert_eq!(unsafe { state() }, 2);
    drop(duplicate);
    assert_eq!(
        lease.mapping().state,
        ImageMappingState::RetainedByLeases(1)
    );
    // Revoke the callback, then retire the *foreign* thread and its destructor.
    finish_thread.send(()).unwrap();
    foreign.join().unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    // SAFETY: foreign TLS has returned; its key still belongs to a mapped image.
    assert_eq!(unsafe { retire() }, 0);
    drop(lease);
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.mappings[0].state, ImageMappingState::Released);
    let after = observer.settled("lease-released", 0, baseline[3]);
    assert_eq!(after[2..5], baseline[2..5]);
    assert_eq!(observer.tls("lease-released"), tls);
    observed(
        progress,
        "observed foreign TLS join then final code lease release",
    );
    eprintln!(
        "MAPPING-LEASE necessary-retention=true foreign-tls=1 callback-revoked=true actual-unmap=true"
    );
}
