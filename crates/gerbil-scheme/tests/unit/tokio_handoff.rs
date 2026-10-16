// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Real Tokio cancellation/admission against the qualified, single-owner ABI.
//! This is transport qualification, not a parser or Scheme SMP benchmark.

use std::{sync::Arc, thread, time::Instant};

use gerbil_scheme::{GerbilRuntime, GerbilStatus, NativeError};
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

#[derive(Debug)]
struct Reply {
    bytes: Vec<u8>,
    queue_ns: u128,
    service_ns: u128,
}

struct Request {
    source: Arc<[u8]>,
    submitted: Instant,
    response: oneshot::Sender<Reply>,
    expected_abandonment: bool,
    barrier: Option<Barrier>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Rooted,
    Released,
}

struct Barrier {
    stage: Stage,
    entered: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
}

fn reach_barrier(barrier: &mut Option<Barrier>, stage: Stage) {
    if barrier
        .as_ref()
        .is_some_and(|barrier| barrier.stage == stage)
    {
        let barrier = barrier.take().expect("matching barrier");
        barrier.entered.send(()).expect("publish owner stage");
        barrier
            .resume
            .blocking_recv()
            .expect("resume admitted work");
    }
}

#[derive(Debug, Default)]
struct Drain {
    completed: usize,
    abandoned: usize,
    released: usize,
    received: usize,
    receive_batches: usize,
    max_batch: usize,
}

fn run_owner(mut incoming: mpsc::Receiver<Request>, ready: oneshot::Sender<()>) -> Drain {
    let runtime = GerbilRuntime::initialize().expect("initialize on the foreign-entry owner");
    ready.send(()).expect("publish runtime readiness");
    let mut drain = Drain::default();
    let limit = incoming.max_capacity();
    let mut pending = Vec::with_capacity(limit);
    while incoming.blocking_recv_many(&mut pending, limit) != 0 {
        drain.received += pending.len();
        drain.receive_batches += 1;
        drain.max_batch = drain.max_batch.max(pending.len());
        for mut request in pending.drain(..) {
            let queue_ns = request.submitted.elapsed().as_nanos();
            let service = Instant::now();
            let root = runtime
                .bytevector_from_bytes(&request.source)
                .expect("Scheme allocation and root publication");
            let token = root.root_id();
            let cancelled = request.expected_abandonment;
            reach_barrier(&mut request.barrier, Stage::Rooted);
            let bytes = root
                .to_vec()
                .into_result()
                .expect("copy before root release");
            drop(root);
            let mut released_length = 0;
            // SAFETY: this owner checks the just-released token before another
            // allocation can reuse it. Count ABI-confirmed releases, not drops.
            assert_eq!(
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_length(
                        token,
                        &raw mut released_length,
                    )
                },
                GerbilStatus::InvalidValue
            );
            drain.released += 1;
            reach_barrier(&mut request.barrier, Stage::Released);
            if cancelled {
                let mut output = vec![0; bytes.len()];
                // SAFETY: only this initialized owner enters the ABI. A released
                // token must be rejected before any host result notification.
                assert_eq!(
                    unsafe {
                        gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                            token,
                            output.as_mut_ptr(),
                            output.len(),
                        )
                    },
                    GerbilStatus::InvalidValue
                );
            }
            let reply = Reply {
                bytes,
                queue_ns,
                service_ns: service.elapsed().as_nanos(),
            };
            if let Err(reply) = request.response.send(reply) {
                assert!(
                    cancelled,
                    "only the intentionally aborted awaiter may disappear"
                );
                assert_eq!(reply.bytes, [0, 255, 127]);
                drain.abandoned += 1;
            }
            drain.completed += 1;
            if drain.completed % 1_000 == 0 {
                eprintln!(
                    "BRIDGE-PROGRESS completed={} released={}",
                    drain.completed, drain.released
                );
            }
        }
    }
    assert_eq!(runtime.add_i64(40, 2).expect("usable after draining"), 42);
    drop(runtime);
    // Runtime teardown is terminal; dropping an awaiter must never perform it.
    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::RuntimeFinalized)
    ));
    drain
}

async fn cancel_at_stage(requests: &mpsc::Sender<Request>, stage: Stage) {
    let (entered, rooted) = oneshot::channel();
    let (resume, resume_owner) = oneshot::channel();
    let (response, result) = oneshot::channel();
    let awaiter = tokio::spawn(result);
    requests
        .send(Request {
            source: Arc::from([0, 255, 127]),
            submitted: Instant::now(),
            response,
            expected_abandonment: true,
            barrier: Some(Barrier {
                stage,
                entered,
                resume: resume_owner,
            }),
        })
        .await
        .unwrap_or_else(|_| panic!("owner accepts cancellation control"));
    rooted
        .await
        .expect("owner reaches requested stage before abort");
    awaiter.abort();
    assert!(
        awaiter
            .await
            .expect_err("aborted task must not deliver")
            .is_cancelled()
    );
    resume.send(()).expect("owner survives task abort");
    eprintln!("BRIDGE-CANCEL stage={stage:?} task-abort=OK");
}

async fn cancel_while_queued(requests: &mpsc::Sender<Request>) {
    let (entered, rooted) = oneshot::channel();
    let (resume, resume_owner) = oneshot::channel();
    let (response, completed) = oneshot::channel();
    requests
        .send(Request {
            source: Arc::from([0, 255, 127]),
            submitted: Instant::now(),
            response,
            expected_abandonment: false,
            barrier: Some(Barrier {
                stage: Stage::Rooted,
                entered,
                resume: resume_owner,
            }),
        })
        .await
        .unwrap_or_else(|_| panic!("owner accepts blocking control"));
    rooted.await.expect("owner is parked with a live root");
    let (response, result) = oneshot::channel();
    let awaiter = tokio::spawn(result);
    requests
        .send(Request {
            source: Arc::from([0, 255, 127]),
            submitted: Instant::now(),
            response,
            expected_abandonment: true,
            barrier: None,
        })
        .await
        .unwrap_or_else(|_| panic!("queued request is admitted"));
    awaiter.abort();
    assert!(
        awaiter
            .await
            .expect_err("queued awaiter is aborted")
            .is_cancelled()
    );
    resume
        .send(())
        .expect("resume owner to drain both requests");
    assert_eq!(
        completed.await.expect("blocking control completes").bytes,
        [0, 255, 127]
    );
    eprintln!("BRIDGE-CANCEL stage=Queued task-abort=OK");
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    assert!(!samples.is_empty());
    samples.sort_unstable();
    samples[(samples.len() * percent).div_ceil(100) - 1]
}

async fn qualify_batch(
    requests: &mpsc::Sender<Request>,
    workers: usize,
    count: usize,
    size: usize,
    pipelined: bool,
) {
    let window = if pipelined {
        (requests.max_capacity() / workers.min(count)).max(1)
    } else {
        1
    };
    let source: Arc<[u8]> = Arc::from(vec![0xA5; size]);
    let expected = vec![0xA5; size];
    // Fixture creation and runtime startup are outside the batch boundary.
    // Timestamp instrumentation, host validation and result disposal are inside.
    let started = Instant::now();
    let mut tasks = JoinSet::new();
    for worker in 0..workers.min(count) {
        let requests = requests.clone();
        let source = Arc::clone(&source);
        let expected = expected.clone();
        tasks.spawn(async move {
            let mut observations = Vec::new();
            let assigned = (count - worker).div_ceil(workers);
            let mut results = Vec::with_capacity(window);
            while observations.len() < assigned {
                let admitted = window.min(assigned - observations.len());
                if pipelined {
                    let submitted = Instant::now(); // include permit wait in queue latency
                    let permits = requests
                        .reserve_many(admitted)
                        .await
                        .expect("bounded window admission");
                    for permit in permits {
                        let (response, result) = oneshot::channel();
                        permit.send(Request {
                            source: Arc::clone(&source),
                            submitted,
                            response,
                            expected_abandonment: false,
                            barrier: None,
                        });
                        results.push((submitted, result));
                    }
                } else {
                    let (response, result) = oneshot::channel();
                    let submitted = Instant::now();
                    requests
                        .send(Request {
                            source: Arc::clone(&source),
                            submitted,
                            response,
                            expected_abandonment: false,
                            barrier: None,
                        })
                        .await
                        .unwrap_or_else(|_| panic!("owner remains live"));
                    results.push((submitted, result));
                }
                for (submitted, result) in results.drain(..) {
                    let reply = result.await.expect("one terminal result");
                    assert_eq!(reply.bytes, expected, "full payload parity");
                    observations.push((
                        reply.queue_ns,
                        reply.service_ns,
                        submitted.elapsed().as_nanos(),
                    ));
                    drop(reply);
                }
            }
            observations
        });
    }
    let mut queue = Vec::with_capacity(count);
    let mut service = Vec::with_capacity(count);
    let mut end_to_end = Vec::with_capacity(count);
    while let Some(task) = tasks.join_next().await {
        for (queue_ns, service_ns, end_to_end_ns) in task.expect("host caller succeeds") {
            queue.push(queue_ns);
            service.push(service_ns);
            end_to_end.push(end_to_end_ns);
        }
    }
    let elapsed = started.elapsed();
    assert_eq!(end_to_end.len(), count);
    eprintln!(
        "BRIDGE-TRANSPORT completed={count} bytes={size} callers={workers} owners=1 window={window} pipelined={pipelined} instrumentation=true elapsed_ns={} queue_p95_ns={} service_p95_ns={} request_p95_ns={}",
        elapsed.as_nanos(),
        percentile(&mut queue, 95),
        percentile(&mut service, 95),
        percentile(&mut end_to_end, 95)
    );
}

#[test]
fn tokio_abort_and_concurrent_batches_drain_before_runtime_cleanup() {
    let workers = thread::available_parallelism()
        .expect("machine CPU budget")
        .get();
    let capacity = workers.checked_mul(2).expect("queue budget fits usize");
    let host = Builder::new_multi_thread()
        .worker_threads(workers)
        .build()
        .expect("Tokio runtime");
    let (requests, incoming) = mpsc::channel(capacity);
    let (ready, started) = oneshot::channel();
    let owner = thread::spawn(move || run_owner(incoming, ready));
    let completed = host.block_on(async {
        started.await.expect("owner is ready before qualification");
        cancel_at_stage(&requests, Stage::Rooted).await;
        cancel_at_stage(&requests, Stage::Released).await;
        cancel_while_queued(&requests).await;
        // Partial reservation abandonment must not strand capacity or an
        // accepted Scheme root. Drop the receiver before committing one item.
        let mut permits = requests.reserve_many(capacity).await.unwrap();
        let (response, result) = oneshot::channel();
        drop(result);
        permits.next().unwrap().send(Request {
            source: Arc::from([0, 255, 127]),
            submitted: Instant::now(),
            response,
            expected_abandonment: true,
            barrier: None,
        });
        drop(permits); // unused reservation is not accepted work
        let mut completed = 5;
        for count in [1_000, 10_000, 100_000] {
            for size in [0, 8_192] {
                qualify_batch(&requests, workers, count, size, false).await;
                completed += count;
            }
        }
        // Uneven totals exercise partial final windows without changing the
        // machine-derived capacity. All accepted roots must still drain.
        for count in [1_003, 10_007, 100_003] {
            for size in [0, 8_192] {
                qualify_batch(&requests, workers, count, size, true).await;
                completed += count;
            }
        }
        completed
    });
    drop(requests); // close admission; owner drains before dropping its runtime
    let drain = owner.join().expect("owner exits after full drain");
    assert_eq!(drain.completed, completed);
    assert_eq!(drain.received, completed);
    assert!(drain.max_batch <= capacity);
    assert_eq!(drain.released, completed);
    assert_eq!(drain.abandoned, 4);
    eprintln!(
        "BRIDGE-DRAIN completed={} released={} abandoned={} receive_batches={} max_batch={} cleanup=OK",
        drain.completed, drain.released, drain.abandoned, drain.receive_batches, drain.max_batch
    );
}

#[test]
fn percentile_uses_nearest_rank_without_rounding_to_zero() {
    assert_eq!(percentile(&mut [9, 1, 5, 3], 95), 9);
    assert_eq!(percentile(&mut [9, 1, 5, 3], 50), 3);
    assert_eq!(percentile(&mut [0], 95), 0);
}
