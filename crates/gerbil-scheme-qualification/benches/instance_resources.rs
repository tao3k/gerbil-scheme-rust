//! Native types, task failures, initialization rollback and fixed-owner reuse.
#[path = "instance_resources/ab.rs"]
mod ab;
#[path = "instance_resources/contracts.rs"]
mod contracts;
#[path = "instance_resources/green.rs"]
mod green;
#[path = "instance_resources/observe.rs"]
mod observe;
#[path = "instance_resources/order_profile.rs"]
mod order_profile;
#[path = "instance_resources/owned_vm.rs"]
mod owned_vm;
#[path = "instance_resources/owner_wake.rs"]
mod owner_wake;
#[path = "instance_resources/orders.rs"]
mod orders;
#[path = "instance_resources/retirement.rs"]
mod retirement;
#[path = "instance_resources/retained.rs"]
mod retained;
#[path = "instance_resources/reuse.rs"]
mod reuse;
#[path = "instance_resources/root_ab.rs"]
mod root_ab;
#[path = "instance_resources/scheduler_ab.rs"]
mod scheduler_ab;
use gerbil_scheme::instances::{InstanceConfig, InstanceDispatcher, InstancePool, IsolatedProgram};
use std::{sync::mpsc, thread, time::Duration};
const GATE: Duration = Duration::from_secs(5);
type Progress = mpsc::Sender<&'static str>;
fn bounded_integer(value: usize) -> i64 {
    i64::try_from(value).expect("bounded qualification integer")
}
fn observed(progress: &Progress, event: &'static str) {
    progress.send(event).unwrap();
}
fn program(path: &str, entry: &std::ffi::CStr) -> IsolatedProgram {
    // SAFETY: compiler-owned images reuse the existing native type/root ABI.
    unsafe { IsolatedProgram::from_image(path, entry) }.unwrap()
}
fn normal() -> IsolatedProgram {
    program(
        gerbil_scheme_qualification::owner_instance_image(),
        c"gerbil_owner_native_call",
    )
}
fn config(owners: usize) -> InstanceConfig {
    InstanceConfig {
        owners,
        queue_capacity: 64,
        maximum_request_bytes: 16 * 1024 * 1024,
        maximum_reply_bytes: 16 * 1024 * 1024,
        startup_timeout: GATE,
    }
}
fn command(
    dispatch: &InstanceDispatcher,
    owner: usize,
    input: Vec<i64>,
    progress: &Progress,
) -> Vec<i64> {
    let value = dispatch
        .submit_native_on(owner, move |session| {
            let input = session.numeric_vector(&input)?;
            let result = session.call(&input)?;
            session.copy_numeric::<i64>(&result)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    observed(progress, "observed native typed response");
    value
}
fn checkpoint(pool: &InstancePool, progress: &Progress) -> Vec<[f64; 4]> {
    let dispatch = pool.dispatcher();
    let mut values = Vec::new();
    for owner in pool.owners() {
        values.push(
            dispatch
                .submit_native_on(owner.owner, |session| {
                    session.collect()?;
                    session.usage()
                })
                .unwrap()
                .wait(GATE)
                .unwrap(),
        );
        observed(progress, "observed owner GC and resource snapshot");
    }
    assert!(values.iter().all(|value| value[3] == 0.0));
    values
}
fn finish(pool: &mut InstancePool, progress: &Progress) {
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
    observed(progress, "observed all owners clean and joined");
    eprintln!(
        "RESOURCE-DRAIN joined={} pending={} roots={:?} native-cleanup-ns={:?} io-retired-bytes={:?} files-removed={:?}",
        receipt.joined,
        receipt.pending,
        receipt
            .owners
            .iter()
            .map(|owner| owner.roots_before_cleanup)
            .collect::<Vec<_>>(),
        receipt
            .owners
            .iter()
            .map(|owner| owner.cleanup_time.as_nanos())
            .collect::<Vec<_>>(),
        receipt
            .owners
            .iter()
            .map(|owner| owner.native_io_released_bytes)
            .collect::<Vec<_>>(),
        receipt
            .owners
            .iter()
            .map(|owner| owner.image_file_removed)
            .collect::<Vec<_>>()
    );
}
fn main() {
    let (progress, events) = mpsc::channel();
    let watchdog = thread::spawn(move || {
        let mut last = "start";
        loop {
            match events.recv_timeout(GATE) {
                Ok(event) => last = event,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("RESOURCE-STALL last={last} cleanup=UNVERIFIED");
                    std::process::exit(70);
                }
            }
        }
    });
    let observer = observe::Observer::new();
    let mode = std::env::args().nth(1).unwrap_or_else(|| "--reuse".into());
    match mode.as_str() {
        "--owned-vm" => owned_vm::run(&progress),
        "--retained" => retained::run(&progress),
        "--owner-wake" => owner_wake::run(&progress),
        "--root-ab" => root_ab::run(&progress),
        "--scheduler-ab" => scheduler_ab::run(&progress),
        "--orders-profile" => order_profile::run(&progress),
        "--orders" => orders::run(&progress),
        "--green-io" => green::parallel_io(&observer, &progress),
        "--green" => green::run(&observer, &progress),
        "--file-failure" => retirement::file_failure(&observer, &progress),
        "--mappings" => retirement::mappings(&observer, &progress),
        "--retirement" => retirement::run(&observer, &progress),
        "--allocator" => retirement::allocator(&observer, &progress),
        "--ab" => ab::run(&progress),
        "--preflight" => contracts::preflight(&observer, &progress),
        "--before" => contracts::initialization(false, &observer, &progress),
        "--after" => contracts::initialization(true, &observer, &progress),
        "--churn" => reuse::churn(&observer, &progress),
        "--smoke" => reuse::run(true, &observer, &progress),
        "--reuse" => reuse::run(false, &observer, &progress),
        _ => panic!("unknown resource qualification mode"),
    }
    drop(progress);
    watchdog.join().unwrap();
}
