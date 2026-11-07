//! Actual fixed-owner Scheme exception, GC, continuation and cleanup checks.
#![cfg(feature = "owner-wait-diagnostic")]
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

#[test]
fn explicit_scope_generation_survives_scheme_gc_exception_and_reentry() {
    let gate = Duration::from_secs(5);
    // SAFETY: the build-owned diagnostic image exposes the existing root ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_wait_diagnostic_image(),
            c"gerbil_owner_orders_call",
        )
        .unwrap()
        .with_exports(&[c"gerbil_owner_service_reference"])
        .with_scheme_service(c"gerbil_owner_service_call")
    };
    let mut pool = InstancePool::start(
        &program,
        InstanceConfig {
            owners: 1,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: gate,
        },
    )
    .unwrap();
    for _ in 0..2 {
        pool.dispatcher()
            .submit_native_on(0, |session| {
                let mut clock = libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                };
                // SAFETY: writable timespec and the same native monotonic clock.
                assert_eq!(
                    unsafe { libc::clock_gettime(libc::CLOCK_UPTIME_RAW, &raw mut clock) },
                    0
                );
                let word = AtomicU64::new(
                    u64::try_from(clock.tv_sec).unwrap() * 1_000_000_000
                        + u64::try_from(clock.tv_nsec).unwrap(),
                );
                let address = (&raw const word) as usize;
                let header = session.numeric_vector::<i64>(&[65, address.try_into().unwrap()])?;
                let request = session.vector(&[&header])?;
                let result = session.call_export(c"gerbil_owner_service_reference", &request)?;
                assert_eq!(session.copy_numeric::<i64>(&result)?, [0]);
                assert!(word.load(Ordering::Acquire) > 0);
                Ok(())
            })
            .unwrap()
            .wait(gate)
            .unwrap();
    }
    let receipt = pool.shutdown(gate);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.owners[0].roots_before_cleanup, 0);
}
