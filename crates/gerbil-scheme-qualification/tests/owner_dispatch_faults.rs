//! Real native owner results survive a Rust submission panic in their peer group.
#![cfg(target_os = "macos")]
use gerbil_scheme::instances::{InstanceConfig, InstanceError, InstancePool, IsolatedProgram};
use gerbil_scheme_python::owner_dispatch::{rayon_owner_submit, tokio_owner_window};
use gerbil_scheme_python::{
    owner_dispatch::WindowBudget,
    owner_flow::{FlowControl, FlowStage, owner_flow_window, owner_flow_window_observed},
};
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
fn check_flow(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    let mut active = 0_usize;
    let mut maximum = 0;
    let mut seen: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    check(runtime.block_on(owner_flow_window_observed(
        jobs(),
        FlowControl {
            budget: WindowBudget {
                released: Instant::now(),
                timeout: GATE,
            },
            maximum_in_flight: 3,
        },
        |owner, id| {
            seen[owner].push(id);
            assert_ne!(id, 3, "injected owner-flow submission panic");
            dispatch.submit_native_async_on(owner, move |session| {
                let value = i64::try_from(id).unwrap();
                let input = session.numeric_vector(&[0, value, value + 1])?;
                let output = session.call(&input)?;
                session.copy_numeric::<i64>(&output)
            })
        },
        |_, stage| match stage {
            FlowStage::Admitted => {
                active += 1;
                maximum = maximum.max(active);
            }
            FlowStage::Collected => active -= 1,
            FlowStage::Submitting => {}
        },
    )));
    assert_eq!(active, 0);
    assert!(maximum <= 3);
    for lane in &seen {
        assert!(lane.windows(2).all(|pair| pair[0] < pair[1]));
    }
    for (limit, released) in [
        (0, Instant::now()),
        (3, Instant::now().checked_sub(GATE).unwrap()),
    ] {
        let mut called = false;
        let rows = runtime.block_on(owner_flow_window_observed(
            jobs(),
            FlowControl {
                budget: WindowBudget {
                    released,
                    timeout: GATE,
                },
                maximum_in_flight: limit,
            },
            |_, _| {
                called = true;
                dispatch.submit_native_async_on(0, |_| Ok(()))
            },
            |_, _| {},
        ));
        assert!(!called, "invalid/expired flow must not admit new work");
        assert_eq!(rows.len(), 17);
        for row in rows {
            if limit == 0 {
                assert_eq!(
                    row.unwrap_err(),
                    InstanceError::InvalidConfig("flow credit must be positive")
                );
            } else {
                assert_eq!(row.unwrap_err(), InstanceError::Timeout);
            }
        }
    }
}
fn check_flow_continuations(pool: &InstancePool, runtime: &tokio::runtime::Runtime) {
    let dispatch = pool.dispatcher();
    for operation in 1..=4 {
        let jobs = (0..4)
            .map(|owner| {
                (
                    owner,
                    vec![2_i64, operation, 100 + i64::try_from(owner).unwrap()],
                )
            })
            .collect();
        let rows = runtime.block_on(owner_flow_window(
            jobs,
            FlowControl {
                budget: WindowBudget {
                    released: Instant::now(),
                    timeout: GATE,
                },
                maximum_in_flight: 4,
            },
            |owner, command| {
                dispatch.submit_native_async_on(owner, move |session| {
                    let input = session.numeric_vector(&command)?;
                    let output = session.call(&input)?;
                    let values = session.copy_numeric::<i64>(&output)?;
                    drop(output);
                    drop(input);
                    session.collect()?;
                    Ok(values)
                })
            },
        ));
        for (owner, row) in rows.into_iter().enumerate() {
            let owner = i64::try_from(owner).unwrap();
            let expected = match operation {
                1 => 23,
                2 => 107 + owner,
                3 => 109 + owner,
                _ => 31,
            };
            assert_eq!(row.unwrap().0, [expected]);
        }
    }
}
fn start(progress: &std::sync::mpsc::Sender<()>) -> InstancePool {
    // SAFETY: the Cargo-owned image implements the existing native root ABI.
    let program = unsafe {
        IsolatedProgram::from_image(
            gerbil_scheme_qualification::owner_instance_image(),
            c"gerbil_owner_native_call",
        )
    }
    .unwrap();
    InstancePool::start_observed(
        &program,
        InstanceConfig {
            owners: 4,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
        |owner| {
            assert_eq!(owner.processors, 1);
            progress.send(()).unwrap();
        },
    )
    .unwrap()
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
    let mut pool = start(&progress);
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
    check_flow(&pool, &runtime);
    check_flow_continuations(&pool, &runtime);
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
