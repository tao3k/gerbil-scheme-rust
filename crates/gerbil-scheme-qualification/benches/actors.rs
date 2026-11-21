use std::cell::RefCell;

use divan::{Bencher, black_box};
use gerbil_scheme::GerbilRuntime;

#[derive(Clone, Copy, Debug)]
struct Load {
    jobs: u32,
    rounds: u32,
    fanout: u32,
}

impl std::fmt::Display for Load {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            output,
            "jobs={}_rounds={}_tasks-per-processor={}",
            self.jobs, self.rounds, self.fanout
        )
    }
}

const LOADS: [Load; 9] = [
    Load {
        jobs: 1_000,
        rounds: 32,
        fanout: 1,
    },
    Load {
        jobs: 1_000,
        rounds: 32,
        fanout: 8,
    },
    Load {
        jobs: 1_000,
        rounds: 32,
        fanout: 64,
    },
    Load {
        jobs: 10_000,
        rounds: 32,
        fanout: 1,
    },
    Load {
        jobs: 10_000,
        rounds: 32,
        fanout: 8,
    },
    Load {
        jobs: 10_000,
        rounds: 32,
        fanout: 64,
    },
    Load {
        jobs: 100_000,
        rounds: 32,
        fanout: 1,
    },
    Load {
        jobs: 100_000,
        rounds: 32,
        fanout: 8,
    },
    Load {
        jobs: 100_000,
        rounds: 32,
        fanout: 64,
    },
];

unsafe extern "C" {
    fn gerbil_actor_qualification_active_processors() -> u32;
    fn gerbil_actor_qualification_processor_count() -> u32;
    fn gerbil_actor_qualification_batch(
        processors: u32,
        actors: u32,
        jobs: u32,
        rounds: u32,
        fail: u32,
    ) -> i64;
}

thread_local! {
    static RUNTIME: RefCell<Option<GerbilRuntime>> = const { RefCell::new(None) };
}

fn main() {
    divan::main();
    RUNTIME.with(|runtime| {
        if let Some(owner) = runtime.borrow_mut().take() {
            drop(owner);
            assert!(matches!(
                GerbilRuntime::initialize(),
                Err(gerbil_scheme::NativeError::RuntimeFinalized)
            ));
            eprintln!("ACTOR-AOT-CLEANUP finalized=OK");
        }
    });
}

fn measure(bencher: Bencher<'_, '_>, load: Load, processors: u32) {
    let jobs = load.jobs;
    let rounds = load.rounds;
    let actors = processors.checked_mul(load.fanout).expect("actor count");
    let expected = i64::from(jobs) * i64::from(rounds) * (i64::from(rounds) + 1) / 2;
    RUNTIME.with(|runtime| {
        let mut owner = runtime.borrow_mut();
        let runtime = owner.get_or_insert_with(|| {
            let program = gerbil_scheme_qualification::linked_program();
            GerbilRuntime::initialize_program(program).expect("full-program owner")
        });
        assert_eq!(runtime.add_i64(40, 2).expect("live owner"), 42);
        let call = |fail| {
            // SAFETY: only this live owner enters the export; Scheme joins all
            // actors and restores processors before returning a scalar result.
            unsafe {
                gerbil_actor_qualification_batch(processors, actors, jobs, rounds, u32::from(fail))
            }
        };
        let result = call(false);
        // SAFETY: the same owner reads the completed activation receipt.
        let active = unsafe { gerbil_actor_qualification_active_processors() };
        assert_eq!(active, processors, "SDK VM processor activation failed; do not admit this as an SMP benchmark");
        assert_eq!(result, expected, "complete checksum preflight");
        eprintln!("EXPECTED-ACTOR-FAILURE processors={processors} actors={actors}");
        assert_eq!(call(true), -1, "worker failure is contained after drain");
        assert_eq!(call(false), expected, "usable after failed batch");
        // SAFETY: the owner reads scalar telemetry after every actor has joined.
        let observed = unsafe { gerbil_actor_qualification_processor_count() };
        assert!(observed > 0 && observed <= processors);
        eprintln!("ACTOR-AOT processors={processors} observed_processors={observed} actors={actors} jobs={jobs} rounds={rounds}");
        bencher.bench_local(|| assert_eq!(black_box(call(false)), expected));
        assert_eq!(runtime.add_i64(40, 2).expect("post-actor owner"), 42);
    });
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn one_processor_control(bencher: Bencher<'_, '_>, load: Load) {
    measure(bencher, load, 1);
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn machine_processors(bencher: Bencher<'_, '_>, load: Load) {
    let processors = u32::try_from(
        std::thread::available_parallelism()
            .expect("CPU budget")
            .get(),
    )
    .expect("processor count fits ABI");
    measure(bencher, load, processors);
}

// Matched synthetic checksum semantics and job partitioning. Keep each
// compiler's ordinary optimizations: this is not isolated scheduler overhead.
fn rust_measure(bencher: Bencher<'_, '_>, load: Load, workers: u32) {
    let jobs = load.jobs;
    let rounds = load.rounds;
    let callers = workers.checked_mul(load.fanout).expect("task count");
    let expected = i64::from(jobs) * i64::from(rounds) * (i64::from(rounds) + 1) / 2;
    let host = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(usize::try_from(workers).expect("worker count"))
        .build()
        .expect("Tokio compute control");
    let batch = || {
        host.block_on(async {
            let mut tasks = tokio::task::JoinSet::new();
            for worker in 0..callers {
                tasks.spawn(async move {
                    let mut sum = 0_i64;
                    for _ in (worker..jobs).step_by(usize::try_from(callers).expect("stride")) {
                        let mut remaining = black_box(rounds);
                        let mut checksum = 0_i64;
                        while remaining > 0 {
                            checksum += i64::from(remaining);
                            remaining -= 1;
                        }
                        sum += black_box(checksum);
                    }
                    sum
                });
            }
            let mut sum = 0;
            while let Some(task) = tasks.join_next().await {
                sum += task.expect("compute control completes");
            }
            sum
        })
    };
    assert_eq!(batch(), expected);
    eprintln!("TOKIO-COMPUTE workers={workers} tasks={callers} jobs={jobs} rounds={rounds}");
    bencher.bench_local(|| assert_eq!(black_box(batch()), expected));
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn tokio_one_worker_control(bencher: Bencher<'_, '_>, load: Load) {
    rust_measure(bencher, load, 1);
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn tokio_machine_workers(bencher: Bencher<'_, '_>, load: Load) {
    let workers = u32::try_from(
        std::thread::available_parallelism()
            .expect("CPU budget")
            .get(),
    )
    .expect("worker count fits ABI");
    rust_measure(bencher, load, workers);
}
