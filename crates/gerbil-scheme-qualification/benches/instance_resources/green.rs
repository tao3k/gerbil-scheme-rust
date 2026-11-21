//! Rust routes bounded turns; persistent Scheme mailbox actors schedule within them.
use super::{
    GATE, Progress, bounded_integer, checkpoint, command, config, finish, normal,
    observe::Observer, observed,
};
use gerbil_scheme::instances::{InstanceError, InstancePool};
use std::time::Instant;
#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    batch: usize,
    active: i64,
    quantum: i64,
}
const CASES: [Case; 5] = [
    Case {
        name: "ffi-one-actor-one",
        batch: 1,
        active: 1,
        quantum: 0,
    },
    Case {
        name: "ffi-batch32-actor-one",
        batch: 32,
        active: 1,
        quantum: 0,
    },
    Case {
        name: "ffi-batch32-actors-four",
        batch: 32,
        active: 4,
        quantum: 0,
    },
    Case {
        name: "ffi-batch32-actors-four-yield1",
        batch: 32,
        active: 4,
        quantum: 1,
    },
    Case {
        name: "ffi-batch32-sequential",
        batch: 32,
        active: 0,
        quantum: 0,
    },
];
struct TurnStats {
    peer_progress: i64,
    latencies: Vec<u128>,
}
fn execute(pool: &InstancePool, case: Case, jobs: usize, progress: &Progress) -> TurnStats {
    let dispatch = pool.dispatcher();
    let mut peer_progress = i64::MAX;
    let mut latencies = Vec::new();
    // All variants admit the same 128 semantic items per cohort. Queue capacity
    // is explicitly 128, including the one-item-per-FFI baseline.
    for cohort in (0..jobs).step_by(128) {
        let mut tickets = Vec::new();
        for start in (cohort..(cohort + 128).min(jobs)).step_by(case.batch) {
            let count = case.batch.min(jobs - start);
            let owner = (start / case.batch) % pool.owners().len();
            let input = [
                6,
                1,
                bounded_integer(start),
                bounded_integer(count),
                64,
                case.quantum,
                case.active,
                -1,
            ];
            let enqueued = Instant::now();
            tickets.push((
                start,
                count,
                enqueued,
                dispatch
                    .submit_native_on(owner, move |session| {
                        let input = session.numeric_vector(&input)?;
                        let output = session.call(&input)?;
                        Ok((session.copy_numeric::<i64>(&output)?, Instant::now()))
                    })
                    .unwrap(),
            ));
        }
        for (start, count, enqueued, ticket) in tickets {
            let (output, completed) = ticket.wait(GATE).unwrap();
            latencies.push(completed.duration_since(enqueued).as_nanos());
            assert_eq!(output.len(), count + 1);
            for (index, value) in output[..count].iter().enumerate() {
                assert_eq!(*value, bounded_integer(start + index) + 64 * 65 / 2);
            }
            peer_progress = peer_progress.min(output[count]);
            observed(progress, "observed validated Scheme mailbox actor turn");
        }
    }
    TurnStats {
        peer_progress,
        latencies,
    }
}
fn contracts(pool: &InstancePool, progress: &Progress) {
    let dispatch = pool.dispatcher();
    for owner in 0..pool.owners().len() {
        assert_eq!(command(&dispatch, owner, vec![6, 0, 4], progress), [4]);
        for failing in 0..4 {
            let ticket = dispatch
                .submit_native_on(owner, move |session| {
                    let input = session.numeric_vector(&[6_i64, 1, 0, 32, 64, 1, 4, failing])?;
                    let output = session.call(&input)?;
                    session.copy_numeric::<i64>(&output)
                })
                .unwrap();
            assert!(matches!(
                ticket.wait(GATE),
                Err(InstanceError::Native { code: 0, .. })
            ));
            let output = command(&dispatch, owner, vec![6, 1, 0, 32, 64, 1, 4, -1], progress);
            assert_eq!(output.len(), 33);
            for (index, value) in output[..32].iter().enumerate() {
                assert_eq!(*value, bounded_integer(index) + 2080);
            }
        }
    }
    checkpoint(pool, progress);
    eprintln!(
        "GREEN-CONTRACT owners={} actors-per-owner=4 native-processors=1 caught-actor-errors={} successful-after-error=true roots=0",
        pool.owners().len(),
        pool.owners().len() * 4
    );
}
pub(super) fn run(observer: &Observer, progress: &Progress) {
    for owners in [1, 4] {
        let mut bounds = config(owners);
        bounds.queue_capacity = 128;
        let mut pool = InstancePool::start(&normal(), bounds).unwrap();
        observed(
            progress,
            "observed fixed owners for mailbox actor qualification",
        );
        let before = observer.emit("green-before-actors", owners, &[]);
        contracts(&pool, progress);
        let after = observer.emit("green-after-actors", owners, &[]);
        assert_eq!(
            before[3], after[3],
            "Scheme actors must not create OS workers"
        );
        for jobs in [512, 4096] {
            for case in CASES {
                execute(&pool, case, jobs, progress);
            }
            let mut samples: [Vec<u128>; 5] = std::array::from_fn(|_| Vec::new());
            let mut latencies: [Vec<u128>; 5] = std::array::from_fn(|_| Vec::new());
            for block in 0..10 {
                for variant in [0, 1, 2, 3, 4, 4, 3, 2, 1, 0] {
                    let start = Instant::now();
                    let stats = execute(&pool, CASES[variant], jobs, progress);
                    let ns = start.elapsed().as_nanos();
                    samples[variant].push(ns);
                    latencies[variant].extend(stats.latencies);
                    let peers = stats.peer_progress;
                    eprintln!(
                        "GREEN-SAMPLE owners={owners} jobs={jobs} block={block} case={} ns={ns} first-peer-progress-min={peers}",
                        CASES[variant].name
                    );
                }
            }
            for (variant, (case, values)) in CASES.iter().zip(&mut samples).enumerate() {
                values.sort_unstable();
                let median = values[9].midpoint(values[10]);
                latencies[variant].sort_unstable();
                let p95 = latencies[variant][(latencies[variant].len() * 95).div_ceil(100) - 1];
                eprintln!(
                    "GREEN-RESULT owners={owners} jobs={jobs} case={} samples=20 median-ns={median} max-ns={} ffi-per-sample={} equal-admitted-items=128 enqueue-projection-p95-ns={p95}",
                    case.name,
                    values[19],
                    jobs / case.batch
                );
            }
        }
        timer_waits(&pool, progress);
        for owner in 0..owners {
            assert_eq!(
                command(&pool.dispatcher(), owner, vec![6, 2], progress),
                [4]
            );
        }
        checkpoint(&pool, progress);
        finish(&mut pool, progress);
    }
}

fn timer_waits(pool: &InstancePool, progress: &Progress) {
    let dispatch = pool.dispatcher();
    let mut samples: [Vec<u128>; 2] = [Vec::new(), Vec::new()];
    for block in 0..10 {
        for concurrent in [0, 1, 1, 0] {
            let start = Instant::now();
            assert_eq!(
                command(
                    &dispatch,
                    0,
                    vec![6, 3, bounded_integer(concurrent), 16],
                    progress
                ),
                [16 * 64 * 65 / 2]
            );
            let ns = start.elapsed().as_nanos();
            samples[concurrent].push(ns);
            eprintln!(
                "GREEN-TIMER block={block} owners={} target-owner=0 concurrent={concurrent} timers=16 ns={ns} validated=true",
                pool.owners().len()
            );
        }
    }
    for (concurrent, values) in samples.iter_mut().enumerate() {
        values.sort_unstable();
        eprintln!(
            "GREEN-TIMER-RESULT owners={} target-owner=0 concurrent={concurrent} samples=20 median-ns={} extra-rust-workers=0",
            pool.owners().len(),
            values[9].midpoint(values[10])
        );
    }
}

pub(super) fn parallel_io(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("parallel-io-before", 0, &[]);
    let mut pool = InstancePool::start(&normal(), config(4)).unwrap();
    observed(progress, "observed parallel I/O native owners initialized");
    let active = observer.emit("parallel-io-active", 0, &[]);
    let dispatch = pool.dispatcher();
    for cohort in 0..16 {
        let start = Instant::now();
        let tickets: Vec<_> = (0..4)
            .map(|owner| {
                dispatch
                    .submit_native_on(owner, |session| {
                        let input = session.numeric_vector(&[6_i64, 3, 1, 16])?;
                        session.collect()?;
                        let result = session.call(&input)?;
                        session.collect()?;
                        session.copy_numeric::<i64>(&result)
                    })
                    .unwrap()
            })
            .collect();
        for ticket in tickets {
            assert_eq!(ticket.wait(GATE).unwrap(), [16 * 64 * 65 / 2]);
            observed(
                progress,
                "observed overlapping native Scheme I/O timers and GC",
            );
        }
        eprintln!(
            "GREEN-IO-COHORT cohort={cohort} owners=4 timers-per-owner=16 validated=4 ns={}",
            start.elapsed().as_nanos()
        );
    }
    checkpoint(&pool, progress);
    let after = observer.emit("parallel-io-after", 0, &[]);
    assert_eq!(
        after[3], active[3],
        "green timer waits must not spawn OS workers"
    );
    finish(&mut pool, progress);
    let ended = observer.settled("parallel-io-drained", 0, baseline[3]);
    assert_eq!(ended[2..4], baseline[2..4]);
    eprintln!(
        "GREEN-IO-CONTRACT owners=4 cohorts=16 timers=1024 roots=0 fd-thread-return=true extra-rust-workers=0"
    );
}
