//! Experimental private runtime images. NOT a supported parallel parser.

#[path = "instances/image.rs"]
mod image;
#[path = "instances/pool.rs"]
mod pool;

use divan::{Bencher, black_box};
use pool::Pool;
use std::{
    cell::Cell,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static GENERATION: AtomicUsize = AtomicUsize::new(0);
const LOADS: [u32; 3] = [1_000, 10_000, 100_000];

fn main() {
    divan::main();
}

fn measure(bencher: Bencher<'_, '_>, jobs: u32, workers: usize, rounds: u32) {
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
    let directory = PathBuf::from(env!("GERBIL_INSTANCE_PROBE_IMAGE"))
        .parent()
        .expect("image output")
        .join(format!("probe-{}-{generation}", std::process::id()));
    std::fs::create_dir(&directory).expect("unique probe image directory");
    let mut paths = Vec::new();
    for worker in 0..workers {
        let path = directory.join(format!("worker-{worker}.dylib"));
        std::fs::copy(env!("GERBIL_INSTANCE_PROBE_IMAGE"), &path).expect("private image copy");
        paths.push(path);
    }
    let pool = Pool::new(paths);
    let expected = i64::from(jobs) * i64::from(rounds) * (i64::from(rounds) + 1) / 2;
    assert_eq!(pool.batch(jobs, rounds, true), expected);
    eprintln!(
        "INSTANCE-LOAD jobs={jobs} rounds={rounds} workers={workers} workload=synthetic-checksum-not-parser"
    );
    let completed = Cell::new(0);
    bencher.bench_local(|| {
        assert_eq!(black_box(pool.batch(jobs, rounds, false)), expected);
        completed.set(completed.get() + 1);
        eprintln!(
            "INSTANCE-SAMPLE jobs={jobs} rounds={rounds} workers={workers} completed-batches={}",
            completed.get()
        );
    });
    drop(pool);
    // Retain image files as diagnostic artifacts. No hot unload or restart
    // admission follows from process-scoped image lifetime.
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn single_owner(bencher: Bencher<'_, '_>, jobs: u32) {
    measure(bencher, jobs, 1, 32);
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn machine_owners(bencher: Bencher<'_, '_>, jobs: u32) {
    measure(
        bencher,
        jobs,
        std::thread::available_parallelism()
            .expect("CPU budget")
            .get(),
        32,
    );
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn cpu_heavy_single_owner(bencher: Bencher<'_, '_>, jobs: u32) {
    measure(bencher, jobs, 1, 4096);
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn cpu_heavy_machine_owners(bencher: Bencher<'_, '_>, jobs: u32) {
    measure(
        bencher,
        jobs,
        std::thread::available_parallelism()
            .expect("CPU budget")
            .get(),
        4096,
    );
}
