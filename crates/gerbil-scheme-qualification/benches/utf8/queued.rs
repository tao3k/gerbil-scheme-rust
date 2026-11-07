//! Tokio closed-loop admission, with exactly one owner-local Scheme codec.
use super::{GerbilRuntime, SCENARIOS, buffer, convert, latency, texts_for_scenario};
use std::{
    sync::{Arc, mpsc as std_mpsc},
    thread,
    time::Instant,
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

struct Request {
    corpus: usize,
    slot: usize,
    enqueued: Instant,
    reply: oneshot::Sender<Reply>,
}
struct Reply {
    text: String,
    queue_ns: u64,
    service_ns: u64,
}
fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap()
}

pub(super) fn run() {
    latency::contracts();
    let workers = thread::available_parallelism()
        .expect("machine CPU budget")
        .get();
    let callers = workers.checked_mul(2).unwrap();
    let host = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .build()
        .unwrap();
    let (send, receive) = mpsc::channel(workers);
    let (ready, wait) = std_mpsc::channel();
    let owner = thread::spawn(move || owner(receive, ready));
    wait.recv().expect("owner ready before clocks");
    println!(
        "UTF8-QUEUED-METHOD scope=bridge workload=closed_loop workers={workers} callers={callers} queue_capacity={workers} scheme_owners=1 parser_parallelism=false units=ns timer_correction=none"
    );
    let scenario = std::env::args().nth(2).filter(|arg| arg != "--bench");
    for (corpus, name) in SCENARIOS.iter().enumerate() {
        if scenario
            .as_deref()
            .is_some_and(|wanted| wanted != "all" && wanted != *name)
        {
            continue;
        }
        let texts = Arc::new(texts_for_scenario(name));
        for jobs in [1_000, 10_000, 100_000] {
            measure(&host, &send, &texts, corpus, jobs, callers);
        }
    }
    drop(send);
    owner.join().expect("owner drains and releases all roots");
    println!("UTF8-QUEUED-COMPLETE batches_per_load=20 concurrent_latency_slo_admission=false");
}

fn owner(mut incoming: mpsc::Receiver<Request>, ready: std_mpsc::Sender<()>) {
    let runtime =
        GerbilRuntime::initialize_program(gerbil_scheme_qualification::linked_program()).unwrap();
    buffer::contracts(&runtime);
    let roots: Vec<Vec<_>> = SCENARIOS
        .iter()
        .map(|name| {
            texts_for_scenario(name)
                .iter()
                .map(|text| buffer::root_text(&runtime, text))
                .collect()
        })
        .collect();
    ready.send(()).unwrap();
    drop(ready);
    while let Some(request) = incoming.blocking_recv() {
        let started = Instant::now();
        let queue_ns = u64::try_from(started.duration_since(request.enqueued).as_nanos()).unwrap();
        let text = convert(&roots[request.corpus][request.slot], 5);
        let service_ns = nanos(started);
        request
            .reply
            .send(Reply {
                text,
                queue_ns,
                service_ns,
            })
            .unwrap_or_else(|_| panic!("client disappeared"));
    }
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    drop(roots);
    drop(runtime);
}

fn measure(
    host: &Runtime,
    send: &mpsc::Sender<Request>,
    texts: &Arc<Vec<String>>,
    corpus: usize,
    jobs: usize,
    callers: usize,
) {
    let name = SCENARIOS[corpus];
    let mut all = Vec::with_capacity(jobs * 20);
    let mut wall_total = 0.0;
    let warmup = host.block_on(batch_requests(send, texts, corpus, 1_000, callers));
    assert_eq!(warmup.len(), 1_000);
    drop(warmup);
    for batch in 0..20 {
        let started = Instant::now();
        let rows = host.block_on(batch_requests(send, texts, corpus, jobs, callers));
        let wall = started.elapsed().as_secs_f64();
        wall_total += wall;
        assert_eq!(rows.len(), jobs);
        println!(
            "UTF8-QUEUED-BATCH corpus={name} jobs={jobs} batch={} requests={jobs} errors=0 batch_wall_ms={:.3} completed_rps={:.3}",
            batch + 1,
            wall * 1000.0,
            f64::from(u32::try_from(jobs).unwrap()) / wall
        );
        report(name, jobs, batch + 1, &rows);
        all.extend(rows);
    }
    assert_eq!(all.len(), jobs * 20);
    println!(
        "UTF8-QUEUED-TOTAL corpus={name} jobs={jobs} batches=20 requests={} errors=0 total_wall_ms={:.3} completed_rps={:.3} sample_capacity_bytes={}",
        all.len(),
        wall_total * 1000.0,
        f64::from(u32::try_from(all.len()).unwrap()) / wall_total,
        all.capacity() * std::mem::size_of::<[u64; 4]>()
    );
    report(name, jobs, 0, &all);
}

async fn batch_requests(
    send: &mpsc::Sender<Request>,
    texts: &Arc<Vec<String>>,
    corpus: usize,
    jobs: usize,
    callers: usize,
) -> Vec<[u64; 4]> {
    let mut tasks = JoinSet::new();
    for caller in 0..callers.min(jobs) {
        let send = send.clone();
        let texts = Arc::clone(texts);
        tasks.spawn(async move {
            let mut rows = Vec::with_capacity(jobs.div_ceil(callers));
            for index in (caller..jobs).step_by(callers) {
                let slot = index % texts.len();
                let (reply, wait) = oneshot::channel();
                let issued = Instant::now();
                let permit = send.reserve().await.expect("bounded admission");
                let enqueued = Instant::now();
                let admission = u64::try_from(enqueued.duration_since(issued).as_nanos()).unwrap();
                permit.send(Request {
                    corpus,
                    slot,
                    enqueued,
                    reply,
                });
                let result = wait.await.expect("live owner response");
                let response = nanos(issued);
                assert_eq!(result.text, texts[slot]);
                assert!(
                    response >= admission + result.queue_ns + result.service_ns,
                    "consistent request time boundaries"
                );
                rows.push([admission, result.queue_ns, result.service_ns, response]);
            }
            rows
        });
    }
    let mut rows = Vec::with_capacity(jobs);
    while let Some(result) = tasks.join_next().await {
        rows.extend(result.expect("all clients complete"));
    }
    rows
}

fn report(corpus: &str, jobs: usize, batch: usize, rows: &[[u64; 4]]) {
    for (field, phase) in ["admission", "queue", "service", "response"]
        .iter()
        .enumerate()
    {
        let mut sorted: Vec<_> = rows.iter().map(|row| row[field]).collect();
        sorted.sort_unstable();
        println!(
            "UTF8-QUEUED-PERCENTILE corpus={corpus} jobs={jobs} batch={batch} phase={phase} observations={} p50_ns={} p95_ns={} p99_ns={} max_ns={}",
            sorted.len(),
            latency::percentile(&sorted, 50),
            latency::percentile(&sorted, 95),
            latency::percentile(&sorted, 99),
            sorted.last().unwrap()
        );
    }
}
