//! Real native owner results survive a Rust submission panic in their peer group.
#![cfg(target_os = "macos")]
use gerbil_scheme::instances::{InstanceConfig, InstanceError, InstancePool, IsolatedProgram};
use gerbil_scheme_python::owner_dispatch::{rayon_owner_submit, tokio_owner_window};
use std::time::{Duration, Instant};
const GATE: Duration = Duration::from_secs(5);
fn jobs() -> Vec<(usize, usize)> {
    (0..17).map(|id| (id % 4, id)).collect()
}
fn check(rows: Vec<Result<(Vec<i64>, Instant), InstanceError>>) {
    assert_eq!(rows.len(), 17);
    for (id, row) in rows.into_iter().enumerate() {
        if id == 3 {
            assert_eq!(row.unwrap_err(), InstanceError::TaskPanicked);
        } else {
            let value = i64::try_from(id).unwrap();
            assert_eq!(row.unwrap().0, [value * 3 + 7, value * 3 + 10]);
        }
    }
}
#[test]
fn submission_panics_drain_native_peers_and_preserve_order() {
    let (progress, events) = std::sync::mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        while events.recv_timeout(GATE).is_ok() {}
        if let Err(std::sync::mpsc::TryRecvError::Disconnected) = events.try_recv() {
            return;
        }
        eprintln!("DISPATCH-FAULT-STALL cleanup=UNVERIFIED");
        std::process::exit(70);
    });
    // SAFETY: the Cargo-owned image implements the existing native root ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_native_call",
        )
    }
    .unwrap();
    let mut pool = InstancePool::start(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
    )
    .unwrap();
    progress.send(()).unwrap();
    let dispatch = pool.dispatcher();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let started = Instant::now();
    check(runtime.block_on(tokio_owner_window(
        jobs(),
        started,
        GATE,
        move |owner, id| {
            assert_ne!(id, 3, "injected dispatcher submission panic");
            dispatch.submit_native_async_on(owner, move |session| {
                let value = i64::try_from(id).unwrap();
                let input = session.numeric_vector(&[0, value, value + 1])?;
                let output = session.call(&input)?;
                session.copy_numeric::<i64>(&output)
            })
        },
    )));
    drop(runtime);
    progress.send(()).unwrap();
    let dispatch = pool.dispatcher();
    let workers = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let started = Instant::now();
    let tickets = rayon_owner_submit(&workers, jobs(), |owner, id| {
        assert_ne!(id, 3, "injected dispatcher submission panic");
        dispatch.submit_native_on(owner, move |session| {
            let value = i64::try_from(id).unwrap();
            let input = session.numeric_vector(&[0, value, value + 1])?;
            let output = session.call(&input)?;
            session.copy_numeric::<i64>(&output)
        })
    });
    check(
        tickets
            .into_iter()
            .map(|t| t.and_then(|t| t.wait_completed(GATE.saturating_sub(started.elapsed()))))
            .collect(),
    );
    drop(workers);
    progress.send(()).unwrap();
    for owner in 0..4 {
        pool.dispatcher()
            .submit_native_on(owner, |session| {
                session.collect()?;
                assert_eq!(session.usage()?[3].to_bits(), 0.0_f64.to_bits());
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
        progress.send(()).unwrap();
    }
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    assert_eq!(receipt.joined, 4);
    assert!(
        receipt
            .owners
            .iter()
            .all(|o| o.roots_before_cleanup == 0 && o.image_file_removed)
    );
    drop(progress);
    watchdog.join().unwrap();
}
