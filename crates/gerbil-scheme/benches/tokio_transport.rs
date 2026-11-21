//! Concurrent host admission into the currently qualified single Scheme owner.
//! Divan has one driver; Tokio callers execute on machine-sized worker threads.

use std::{cell::RefCell, sync::Arc, thread};

use divan::{Bencher, black_box};
use gerbil_scheme::{BytestringDelimiter, GerbilRuntime};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

#[path = "transport/load.rs"]
mod load;
use load::{LOADS, Load};

const REQUESTS: usize = 1_000;

#[derive(Clone)]
enum Source {
    Hex(Arc<str>),
    Binary(Arc<[u8]>),
    RustCopy(Arc<[u8]>),
}

struct Request {
    source: Source,
    reply: oneshot::Sender<Vec<u8>>,
}

struct Transport {
    host: Runtime,
    requests: Option<mpsc::Sender<Request>>,
    owner: Option<thread::JoinHandle<Drain>>,
    workers: usize,
}

#[derive(Default)]
struct Drain {
    completed: usize,
    receive_batches: usize,
    max_batch: usize,
}

impl Transport {
    fn new() -> Self {
        let workers = thread::available_parallelism().expect("CPU budget").get();
        let host = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .build()
            .expect("Tokio multi-thread runtime");
        let (requests, mut incoming) = mpsc::channel::<Request>(workers * 2);
        let (ready, initialized) = oneshot::channel();
        let owner = thread::spawn(move || {
            let runtime = GerbilRuntime::initialize().expect("initialize Scheme owner");
            ready.send(()).expect("owner readiness");
            let mut drain = Drain::default();
            let limit = incoming.max_capacity();
            let mut pending = Vec::with_capacity(limit);
            // Official receive-many drains currently available messages; it
            // does not wait for a full batch. Reuse storage and preserve FIFO.
            while incoming.blocking_recv_many(&mut pending, limit) != 0 {
                drain.receive_batches += 1;
                drain.max_batch = drain.max_batch.max(pending.len());
                for request in pending.drain(..) {
                    let bytes = match &request.source {
                        Source::RustCopy(source) => source.to_vec(),
                        Source::Hex(source) => {
                            let root = runtime
                                .bytevector_from_bytestring(source, BytestringDelimiter::Compact)
                                .expect("hex input");
                            root.to_vec().into_result().expect("owned host result")
                        }
                        Source::Binary(source) => {
                            let root = runtime.bytevector_from_bytes(source).expect("binary input");
                            root.to_vec().into_result().expect("owned host result")
                        }
                    }; // release before publication; no handle crosses threads
                    request.reply.send(bytes).expect("live benchmark awaiter");
                    drain.completed += 1;
                }
            }
            assert_eq!(
                runtime
                    .add_i64(40, 2)
                    .expect("drained owner remains usable"),
                42
            );
            drop(runtime);
            drain
        });
        host.block_on(initialized)
            .expect("owner ready before timing");
        eprintln!(
            "TRANSPORT workers={workers} queue_capacity={} scheme_owners=1",
            workers * 2
        );
        Self {
            host,
            requests: Some(requests),
            owner: Some(owner),
            workers,
        }
    }

    fn batch(&self, source: &Source, size: usize, callers: usize, count: usize, pipelined: bool) {
        let requests = self.requests.as_ref().expect("admission open");
        let window = if pipelined {
            (requests.max_capacity() / callers.min(count)).max(1)
        } else {
            1
        };
        self.host.block_on(async {
            let mut tasks = JoinSet::new();
            for caller in 0..callers.min(count) {
                let requests = requests.clone();
                let source = source.clone();
                tasks.spawn(async move {
                    let mut completed = 0;
                    let assigned = (count - caller).div_ceil(callers);
                    let mut results = Vec::with_capacity(window);
                    while completed < assigned {
                        let admitted = window.min(assigned - completed);
                        if pipelined {
                            let permits = requests
                                .reserve_many(admitted)
                                .await
                                .expect("bounded batch admission");
                            for permit in permits {
                                let (reply, result) = oneshot::channel();
                                permit.send(Request {
                                    source: source.clone(),
                                    reply,
                                });
                                results.push(result);
                            }
                        } else {
                            let (reply, result) = oneshot::channel();
                            requests
                                .send(Request {
                                    source: source.clone(),
                                    reply,
                                })
                                .await
                                .unwrap_or_else(|_| panic!("owner admission"));
                            results.push(result);
                        }
                        for result in results.drain(..) {
                            let bytes = result.await.expect("one terminal result");
                            assert_eq!(bytes.len(), size);
                            black_box(&bytes);
                            drop(bytes);
                            completed += 1;
                        }
                    }
                    completed
                });
            }
            let mut completed = 0;
            while let Some(task) = tasks.join_next().await {
                completed += task.expect("caller completes");
            }
            assert_eq!(completed, count);
        });
    }

    fn preflight(&self, source: &Source, size: usize) {
        self.host.block_on(async {
            let (reply, result) = oneshot::channel();
            self.requests
                .as_ref()
                .expect("open admission")
                .send(Request {
                    source: source.clone(),
                    reply,
                })
                .await
                .unwrap_or_else(|_| panic!("preflight admission"));
            assert_eq!(result.await.expect("preflight result"), vec![0xA5; size]);
        });
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        drop(self.requests.take());
        let drain = self
            .owner
            .take()
            .expect("owner join handle")
            .join()
            .expect("drain before terminal runtime cleanup");
        eprintln!(
            "TRANSPORT-DRAIN completed={} receive_batches={} max_batch={} cleanup=OK",
            drain.completed, drain.receive_batches, drain.max_batch
        );
    }
}

thread_local! {
    static TRANSPORT: RefCell<Option<Transport>> = const { RefCell::new(None) };
}

fn main() {
    divan::main();
    TRANSPORT.with(|transport| {
        drop(transport.borrow_mut().take());
    });
}

fn measure(bencher: Bencher<'_, '_>, size: usize, scale: usize) {
    measure_source(
        bencher,
        &Source::Hex(Arc::from("A5".repeat(size))),
        size,
        scale,
        REQUESTS,
    );
}

fn measure_source(
    bencher: Bencher<'_, '_>,
    source: &Source,
    size: usize,
    scale: usize,
    count: usize,
) {
    measure_admission(bencher, source, size, scale, count, false);
}

fn measure_admission(
    bencher: Bencher<'_, '_>,
    source: &Source,
    size: usize,
    scale: usize,
    count: usize,
    pipelined: bool,
) {
    TRANSPORT.with(|transport| {
        let mut fixture = transport.borrow_mut();
        let transport = fixture.get_or_insert_with(Transport::new);
        let callers = if scale == 0 {
            1
        } else {
            transport.workers * scale
        };
        transport.preflight(source, size);
        let window = if pipelined { (transport.requests.as_ref().unwrap().max_capacity() / callers.min(count)).max(1) } else { 1 };
        eprintln!("TRANSPORT-CASE callers={callers} bytes={size} requests_per_batch={count} window={window} pipelined={pipelined}");
        bencher.bench_local(|| transport.batch(source, size, callers, count, pipelined));
    });
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn binary_pipelined_callers(bencher: Bencher<'_, '_>, load: Load) {
    measure_admission(
        bencher,
        &Source::Binary(Arc::from(vec![0xA5; load.bytes])),
        load.bytes,
        1,
        load.requests,
        true,
    );
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn rust_binary_pipelined_control(bencher: Bencher<'_, '_>, load: Load) {
    measure_admission(
        bencher,
        &Source::RustCopy(Arc::from(vec![0xA5; load.bytes])),
        load.bytes,
        1,
        load.requests,
        true,
    );
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn binary_machine_sized_callers(bencher: Bencher<'_, '_>, load: Load) {
    measure_source(
        bencher,
        &Source::Binary(Arc::from(vec![0xA5; load.bytes])),
        load.bytes,
        1,
        load.requests,
    );
}

#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn binary_oversubscribed_callers(bencher: Bencher<'_, '_>, load: Load) {
    measure_source(
        bencher,
        &Source::Binary(Arc::from(vec![0xA5; load.bytes])),
        load.bytes,
        2,
        load.requests,
    );
}

// Same queue, payload, output allocation, notification and disposal. No Scheme
// input allocation/rooting/output copy. This isolates host transport overhead.
#[divan::bench(args = LOADS, sample_count = 20, sample_size = 1, threads = 1)]
fn rust_binary_machine_sized_control(bencher: Bencher<'_, '_>, load: Load) {
    measure_source(
        bencher,
        &Source::RustCopy(Arc::from(vec![0xA5; load.bytes])),
        load.bytes,
        1,
        load.requests,
    );
}

#[divan::bench(args = [0, 8192], sample_count = 20, sample_size = 1, threads = 1)]
fn single_caller_control(bencher: Bencher<'_, '_>, size: usize) {
    measure(bencher, size, 0);
}

#[divan::bench(args = [0, 8192], sample_count = 20, sample_size = 1, threads = 1)]
fn machine_sized_callers(bencher: Bencher<'_, '_>, size: usize) {
    measure(bencher, size, 1);
}

#[divan::bench(args = [0, 8192], sample_count = 20, sample_size = 1, threads = 1)]
fn oversubscribed_callers(bencher: Bencher<'_, '_>, size: usize) {
    measure(bencher, size, 2);
}
