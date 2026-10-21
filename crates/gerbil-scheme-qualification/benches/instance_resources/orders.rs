//! Public batch API, real native order records, matched semantic admission.
#[path = "../support/orders.rs"]
mod corpus;
use super::{GATE, Progress, checkpoint, command, config, finish, observed, program};
pub(super) use corpus::{order, peers, percentile, verify};
use gerbil_scheme::instances::{InstanceError, InstancePool, InstanceTicket};
use std::time::Instant;
pub(super) fn orders_program() -> gerbil_scheme::instances::IsolatedProgram {
    program(
        &std::env::var("GERBIL_ORDERS_IMAGE")
            .unwrap_or_else(|_| gerbil_scheme_qualification::owner_instance_image().to_owned()),
        c"gerbil_owner_orders_call",
    )
}
const JOBS: usize = 1024;
const CASES: [(usize, usize); 4] = [(1, 0), (8, 1), (32, 0), (32, 1)];
pub(super) struct Sample {
    pub(super) elapsed: u128,
    pub(super) latency: Vec<u128>,
    pub(super) probe: Vec<u128>,
    pub(super) peer_min: usize,
}
pub(super) fn sample(
    pool: &InstancePool,
    batch: usize,
    quantum: usize,
    mixed: bool,
    progress: &Progress,
) -> Sample {
    let dispatch = pool.dispatcher();
    sample_using(
        pool,
        batch,
        quantum,
        mixed,
        progress,
        |owner, items, quantum| {
            dispatch
                .submit_numeric_batch_on(owner, items, quantum)
                .unwrap()
        },
    )
}
pub(super) fn sample_using(
    pool: &InstancePool,
    batch: usize,
    quantum: usize,
    mixed: bool,
    progress: &Progress,
    submit: impl Fn(usize, Vec<Vec<i64>>, usize) -> InstanceTicket<Vec<Vec<i64>>>,
) -> Sample {
    let mut result = Sample {
        elapsed: 0,
        latency: Vec::new(),
        probe: Vec::new(),
        peer_min: usize::MAX,
    };
    let start = Instant::now();
    // Every case releases 128 identical orders at once. All input records already
    // exist at release. The one-item control fits the same 128-request queue.
    for base in (0..JOBS).step_by(128) {
        let inputs: Vec<_> = (base..base + 128).map(|id| order(id, mixed)).collect();
        let released = Instant::now();
        let mut pending = Vec::new();
        for (chunk, items) in inputs.chunks(batch).enumerate() {
            let owner = (base + chunk * batch) / 32 % pool.owners().len();
            let ticket = submit(owner, items.to_vec(), quantum);
            pending.push((chunk * batch, ticket));
        }
        for (offset, ticket) in pending {
            let (output, completed) = ticket.wait_completed(GATE).unwrap();
            let latency = completed.duration_since(released).as_nanos();
            result.peer_min = result.peer_min.min(peers(&output));
            assert_eq!(output.len(), batch);
            for (index, row) in output.iter().enumerate() {
                verify(&inputs[offset + index], row);
                result.latency.push(latency);
                if (base + offset + index) % 32 == 31 {
                    result.probe.push(latency);
                }
            }
            observed(progress, "observed native public order batch validated");
        }
    }
    result.elapsed = start.elapsed().as_nanos();
    result
}
fn contracts(pool: &InstancePool, progress: &Progress) {
    let dispatch = pool.dispatcher();
    for count in [0, 257] {
        assert!(matches!(
            dispatch.submit_numeric_batch_on::<i64>(0, vec![vec![0]; count], 1),
            Err(InstanceError::InvalidConfig(_))
        ));
    }
    assert!(
        dispatch
            .submit_numeric_batch_on(0, vec![order(0, false)], 257)
            .is_err()
    );
    assert!(
        dispatch
            .submit_numeric_batch_on(0, vec![vec![0_i64; 3 * 1024 * 1024]], 1)
            .is_err()
    );
    for owner in 0..pool.owners().len() {
        for bad in [0, 1, 2, 3, 31] {
            let mut items: Vec<_> = (0..32).map(|id| order(id, false)).collect();
            items[bad][3] = -1;
            assert!(
                dispatch
                    .submit_numeric_batch_on(owner, items, 1)
                    .unwrap()
                    .wait(GATE)
                    .is_err()
            );
            let input = order(1, false);
            let output = dispatch
                .submit_numeric_batch_on(owner, vec![input.clone()], 1)
                .unwrap()
                .wait(GATE)
                .unwrap();
            verify(&input, &output[0]);
            observed(
                progress,
                "observed Scheme actor exception drained and recovered",
            );
        }
    }
    let inputs: Vec<_> = (0..256).map(|id| order(id, false)).collect();
    let output = dispatch
        .submit_numeric_batch_on(0, inputs.clone(), 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
    for (input, row) in inputs.iter().zip(&output) {
        verify(input, row);
    }
    assert_eq!(output.len(), 256);
    checkpoint(pool, progress);
    eprintln!("ORDERS-CONTRACT bounds=true exception-workers=all recovery=true roots=0");
}
pub(super) fn run(progress: &Progress) {
    let raw_latencies = std::env::var_os("GERBIL_ORDER_LATENCIES").is_some();
    for owners in [1, 4] {
        let mut settings = config(owners);
        settings.queue_capacity = 128;
        let started = Instant::now();
        let mut pool = InstancePool::start(&orders_program(), settings).unwrap();
        eprintln!(
            "ORDERS-SETUP owners={owners} ns={}",
            started.elapsed().as_nanos()
        );
        observed(progress, "observed public batch native owners initialized");
        contracts(&pool, progress);
        for mixed in [false, true] {
            for (batch, quantum) in CASES {
                sample(&pool, batch, quantum, mixed, progress);
            }
            let mut samples: [Vec<u128>; 4] = std::array::from_fn(|_| Vec::new());
            let mut tails: [Vec<u128>; 4] = std::array::from_fn(|_| Vec::new());
            let mut probes: [Vec<u128>; 4] = std::array::from_fn(|_| Vec::new());
            let mut fairness = [usize::MAX; 4];
            // Five forward/reverse blocks: ten measured samples per case.
            for block in 0..5 {
                for case in [0, 1, 2, 3, 3, 2, 1, 0] {
                    let (batch, quantum) = CASES[case];
                    let mut value = sample(&pool, batch, quantum, mixed, progress);
                    if raw_latencies {
                        eprintln!(
                            "ORDER-LATENCIES owners={owners} mixed={mixed} batch={batch} quantum={quantum} block={block} ns={:?}",
                            value.latency
                        );
                    }
                    eprintln!(
                        "ORDERS-SAMPLE owners={owners} mixed={mixed} block={block} batch={batch} quantum={quantum} jobs={JOBS} ns={} p95-ns={} p99-ns={} peer-min={}",
                        value.elapsed,
                        percentile(&mut value.latency, 95),
                        percentile(&mut value.latency, 99),
                        value.peer_min
                    );
                    samples[case].push(value.elapsed);
                    tails[case].extend(value.latency);
                    probes[case].extend(value.probe);
                    fairness[case] = fairness[case].min(value.peer_min);
                }
            }
            for (case, (batch, quantum)) in CASES.iter().enumerate() {
                let median = percentile(&mut samples[case], 50);
                eprintln!(
                    "ORDERS-RESULT owners={owners} mixed={mixed} batch={batch} quantum={quantum} samples=10 jobs={JOBS} median-ns={median} p95-ns={} p99-ns={} probe-p99-ns={} peer-min={}",
                    percentile(&mut tails[case], 95),
                    percentile(&mut tails[case], 99),
                    percentile(&mut probes[case], 99),
                    fairness[case]
                );
            }
        }
        for owner in 0..owners {
            assert_eq!(command(&pool.dispatcher(), owner, vec![7], progress), [0]);
        }
        checkpoint(&pool, progress);
        finish(&mut pool, progress);
    }
    output_bound(progress);
}

fn output_bound(progress: &Progress) {
    let mut settings = config(1);
    settings.maximum_reply_bytes = 21 * 8;
    let mut pool = InstancePool::start(&orders_program(), settings).unwrap();
    observed(progress, "observed small-output-bound owner initialized");
    let dispatch = pool.dispatcher();
    let input = order(0, false);
    let error = dispatch
        .submit_numeric_batch_on(0, vec![input.clone(); 2], 1)
        .unwrap()
        .wait(GATE)
        .unwrap_err();
    assert_eq!(error, InstanceError::InvalidConfig("native output bound"));
    let output = dispatch
        .submit_numeric_batch_on(0, vec![input.clone()], 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
    verify(&input, &output[0]);
    command(&dispatch, 0, vec![7], progress);
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
    eprintln!("ORDERS-OUTPUT-BOUND aggregate=true before-allocation=true recovery=true roots=0");
}
