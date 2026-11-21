//! Same-owner ABBA admission controls; collect every regression before rejection.
use super::{GerbilRuntime, SCENARIOS, buffer, convert, latency, texts_for_scenario};
use std::{
    sync::{Arc, mpsc as std_mpsc},
    thread,
    time::Instant,
};
#[path = "wake_probe.rs"]
mod wake_probe;
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use wake_probe::observe;

pub(super) fn run() {
    latency::contracts();
    let workers = thread::available_parallelism().unwrap().get();
    let callers = workers.checked_mul(2).unwrap();
    let host = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .build()
        .unwrap();
    let (send, receive) = mpsc::channel(workers);
    let (ready, wait) = std_mpsc::channel();
    let owner = thread::spawn(move || owner(receive, ready));
    wait.recv().unwrap();
    println!(
        "UTF8-SCHED-AUDIT-METHOD workers={workers} callers={callers} capacity={workers} scheme_owners=1 workload=closed_loop parser_parallelism=false samples_per_variant=20 comparator=ordinary_observation candidate=perturbed_wake_cpu_observation optimization_admission=false"
    );
    let requested = std::env::args().nth(2).filter(|arg| arg != "--bench");
    let mut admitted = true;
    for (corpus, name) in SCENARIOS.iter().enumerate() {
        if requested
            .as_deref()
            .is_some_and(|s| s != "all" && s != *name)
        {
            continue;
        }
        let texts = Arc::new(texts_for_scenario(name));
        for jobs in [1_000, 10_000, 100_000] {
            admitted &= compare(&host, &send, &texts, corpus, jobs, callers);
        }
    }
    drop(send);
    owner.join().unwrap();
    println!(
        "UTF8-SCHED-AUDIT-COMPLETE optimization_admission=false observer_not_slower={admitted}"
    );
    println!(
        "UTF8-SCHED-AUDIT-BOUNDARY optimized=false instrumentation_slower={}",
        !admitted
    );
}

fn quantiles(rows: &[[u64; 16]]) -> [u64; 8] {
    let mut result = [0; 8];
    for field in 0..4 {
        let mut sorted: Vec<_> = rows.iter().map(|row| row[field]).collect();
        sorted.sort_unstable();
        for (column, p) in [95, 99].iter().enumerate() {
            result[field * 2 + column] = latency::percentile(&sorted, *p);
        }
    }
    result
}

fn compare(
    host: &Runtime,
    send: &mpsc::Sender<Request>,
    texts: &Arc<Vec<String>>,
    corpus: usize,
    jobs: usize,
    callers: usize,
) -> bool {
    let name = SCENARIOS[corpus];
    let mut records: [Vec<[f64; 9]>; 2] = std::array::from_fn(|_| Vec::new());
    let mut paired: [Vec<f64>; 9] = std::array::from_fn(|_| Vec::new());
    let mut all: [Vec<[u64; 16]>; 2] = std::array::from_fn(|_| Vec::with_capacity(jobs * 20));
    for batched in [false, true] {
        assert_eq!(
            host.block_on(batch_requests(send, texts, corpus, 1_000, callers, batched))
                .len(),
            1_000
        );
    }
    for group in 0..10 {
        let mut samples = [[0.0; 9]; 4];
        for (position, mode) in [0, 1, 1, 0].into_iter().enumerate() {
            let os_start = process_usage();
            let start = Instant::now();
            let rows = host.block_on(batch_requests(
                send,
                texts,
                corpus,
                jobs,
                callers,
                mode == 1,
            ));
            let wall = start.elapsed().as_secs_f64();
            let os_end = process_usage();
            audit_batch(host, name, jobs, mode, &rows, os_start, os_end);
            assert_eq!(rows.len(), jobs);
            let q = quantiles(&rows);
            let mut record = [0.0; 9];
            record[0] = wall;
            for (field, value) in q.iter().enumerate() {
                record[field + 1] = std::time::Duration::from_nanos(*value).as_secs_f64();
            }
            println!(
                "UTF8-SCHED-AUDIT-BATCH corpus={name} jobs={jobs} group={group} position={position} mode={mode} requests={jobs} errors=0 wall_ms={:.3} requests_per_second={:.3} admission_p95_ns={} admission_p99_ns={} queue_p95_ns={} queue_p99_ns={} service_p95_ns={} service_p99_ns={} response_p95_ns={} response_p99_ns={}",
                wall * 1000.0,
                f64::from(u32::try_from(jobs).unwrap()) / wall,
                q[0],
                q[1],
                q[2],
                q[3],
                q[4],
                q[5],
                q[6],
                q[7]
            );
            records[mode].push(record);
            samples[position] = record;
            all[mode].extend(rows);
        }
        for field in 0..9 {
            paired[field].push(samples[1][field] / samples[0][field]);
            paired[field].push(samples[2][field] / samples[3][field]);
        }
    }
    for mode in 0..2 {
        assert_eq!(records[mode].len(), 20);
        assert_eq!(all[mode].len(), jobs * 20);
        let q = quantiles(&all[mode]);
        let wall_total: f64 = records[mode].iter().map(|row| row[0]).sum();
        println!(
            "UTF8-SCHED-AUDIT-SUMMARY corpus={name} jobs={jobs} mode={mode} batches=20 requests={} total_wall_ms={:.3} requests_per_second={:.3} sample_capacity_bytes={} admission_p95_ns={} admission_p99_ns={} queue_p95_ns={} queue_p99_ns={} service_p95_ns={} service_p99_ns={} response_p95_ns={} response_p99_ns={}",
            all[mode].len(),
            wall_total * 1000.0,
            f64::from(u32::try_from(all[mode].len()).unwrap()) / wall_total,
            all[mode].capacity() * std::mem::size_of::<[u64; 16]>(),
            q[0],
            q[1],
            q[2],
            q[3],
            q[4],
            q[5],
            q[6],
            q[7]
        );
    }
    let mut admitted = true;
    for (field, metric) in [
        "wall",
        "admission_p95",
        "admission_p99",
        "queue_p95",
        "queue_p99",
        "service_p95",
        "service_p99",
        "response_p95",
        "response_p99",
    ]
    .iter()
    .enumerate()
    {
        let ratio = super::median(&paired[field]);
        admitted &= ratio <= 1.0;
        println!(
            "UTF8-SCHED-AUDIT-DECISION corpus={name} jobs={jobs} metric={metric} samples=20 paired_ratio={ratio:.6} slower={}",
            ratio > 1.0
        );
    }
    admitted
}

unsafe extern "C" {
    fn gerbil_utf8_request_snapshot(slot: i32);
    fn gerbil_utf8_request_gc_stat(field: i32) -> f64;
    fn gerbil_utf8_os_stat(field: i32) -> f64;
    fn gerbil_utf8_process_usage(
        cpu: *mut u64,
        rss: *mut u64,
        vol: *mut u64,
        invol: *mut u64,
    ) -> i32;
}
fn process_usage() -> [u64; 4] {
    let [mut cpu, mut rss, mut vol, mut invol] = [0; 4];
    // SAFETY: independent writable outputs, read-only process OS accounting.
    assert_eq!(
        unsafe {
            gerbil_utf8_process_usage(&raw mut cpu, &raw mut rss, &raw mut vol, &raw mut invol)
        },
        0
    );
    [cpu, rss, vol, invol]
}
struct Request {
    corpus: usize,
    slot: usize,
    enqueued: Instant,
    reply: oneshot::Sender<Reply>,
    diagnostic: bool,
}
struct Reply {
    text: String,
    queue_ns: u64,
    service_ns: u64,
    ready: Instant,
    counters: [u64; 5],
}
fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap()
}
fn counter(value: f64) -> u64 {
    assert!(value.is_finite() && value >= 0.0);
    std::time::Duration::from_secs_f64(value)
        .as_nanos()
        .try_into()
        .unwrap()
}
fn integer_counter(value: f64) -> u64 {
    assert!(value.is_finite() && value >= 0.0 && value.fract() == 0.0);
    // Duration provides a checked, nonallocating float-to-integral conversion.
    std::time::Duration::from_secs_f64(value).as_secs()
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
        let start = Instant::now();
        let queue_ns = u64::try_from(start.duration_since(request.enqueued).as_nanos()).unwrap();
        // SAFETY: snapshots are touched by this Scheme owner only.
        if request.diagnostic {
            unsafe { gerbil_utf8_request_snapshot(0) }
        }
        let text = convert(&roots[request.corpus][request.slot], 5);
        let mut counters = [0; 5];
        if request.diagnostic {
            // SAFETY: matching owner-local snapshots, no cross-thread Scheme data.
            unsafe {
                gerbil_utf8_request_snapshot(1);
                counters = [
                    counter(gerbil_utf8_os_stat(4)),
                    counter(gerbil_utf8_request_gc_stat(2)),
                    integer_counter(gerbil_utf8_os_stat(2)),
                    integer_counter(gerbil_utf8_os_stat(3)),
                    integer_counter(gerbil_utf8_request_gc_stat(0)),
                ];
            }
        }
        let ready = Instant::now();
        let service_ns = u64::try_from(ready.duration_since(start).as_nanos()).unwrap();
        request
            .reply
            .send(Reply {
                text,
                queue_ns,
                service_ns,
                ready,
                counters,
            })
            .unwrap_or_else(|_| panic!("client disappeared"));
    }
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    drop(roots);
    drop(runtime);
}
async fn batch_requests(
    send: &mpsc::Sender<Request>,
    texts: &Arc<Vec<String>>,
    corpus: usize,
    jobs: usize,
    callers: usize,
    diagnostic: bool,
) -> Vec<[u64; 16]> {
    let mut tasks = JoinSet::new();
    for caller in 0..callers.min(jobs) {
        let send = send.clone();
        let texts = Arc::clone(texts);
        tasks.spawn(async move {
            let mut rows = Vec::with_capacity(jobs.div_ceil(callers));
            let mut probe = None;
            for index in (caller..jobs).step_by(callers) {
                let slot = index % texts.len();
                let issued = Instant::now();
                let (reply, wait) = oneshot::channel();
                let permit = send.reserve().await.expect("bounded admission");
                let enqueued = Instant::now();
                let admission = u64::try_from(enqueued.duration_since(issued).as_nanos()).unwrap();
                permit.send(Request {
                    corpus,
                    slot,
                    enqueued,
                    reply,
                    diagnostic,
                });
                let (result, wake) = if diagnostic {
                    observe(async { wait.await.expect("live owner") }, &mut probe).await
                } else {
                    (wait.await.expect("live owner"), [0; 6])
                };
                let response = nanos(issued);
                let handoff = nanos(result.ready);
                assert_eq!(result.text, texts[slot]);
                assert!(response >= admission + result.queue_ns + result.service_ns);
                rows.push([
                    admission,
                    result.queue_ns,
                    result.service_ns,
                    response,
                    handoff,
                    result.counters[0],
                    result.counters[1],
                    result.counters[2],
                    result.counters[3],
                    wake[0],
                    wake[1],
                    wake[2],
                    result.counters[4],
                    wake[3],
                    wake[4],
                    wake[5],
                ]);
            }
            rows
        });
    }
    let mut rows = Vec::with_capacity(jobs);
    while let Some(result) = tasks.join_next().await {
        rows.extend(result.expect("clients complete"));
    }
    rows
}
fn audit_batch(
    host: &Runtime,
    name: &str,
    jobs: usize,
    mode: usize,
    rows: &[[u64; 16]],
    before: [u64; 4],
    after: [u64; 4],
) {
    let complete: Vec<_> = rows.iter().filter(|row| row[15] == 1).collect();
    if !complete.is_empty() {
        let mut sync: Vec<_> = complete.iter().map(|row| row[13]).collect();
        let mut resume: Vec<_> = complete.iter().map(|row| row[14]).collect();
        sync.sort_unstable();
        resume.sort_unstable();
        println!(
            "UTF8-WAKE-PHASE corpus={name} jobs={jobs} mode={mode} complete={} excluded={} synchronous_wake_p95_ns={} synchronous_wake_p99_ns={} wake_to_ready_poll_p95_ns={} wake_to_ready_poll_p99_ns={} diagnostic_only=true mutex_isolation=false",
            complete.len(),
            rows.len() - complete.len(),
            latency::percentile(&sync, 95),
            latency::percentile(&sync, 99),
            latency::percentile(&resume, 95),
            latency::percentile(&resume, 99)
        );
    }
    for (index, row) in rows.iter().enumerate() {
        if mode == 1 && (row[2] >= 1_000_000 || row[4] >= 1_000_000) {
            println!(
                "UTF8-SCHED-AUDIT-SPIKE corpus={name} jobs={jobs} index={index} service_ns={} handoff_ns={} owner_cpu_ns={} gc_wall_ns={} process_voluntary_switches_in_service={} process_involuntary_switches_in_service={} reply_polls={} reply_pending={} forwarded_wakes_in_poll_window={} gc_count={}",
                row[2], row[4], row[5], row[6], row[7], row[8], row[9], row[10], row[11], row[12]
            );
        }
    }
    let mut handoff: Vec<_> = rows.iter().map(|r| r[4]).collect();
    handoff.sort_unstable();
    let mut owner_cpu: Vec<_> = rows.iter().map(|r| r[5]).collect();
    owner_cpu.sort_unstable();
    let totals: [u64; 8] = std::array::from_fn(|i| rows.iter().map(|r| r[i + 5]).sum());
    let gap: usize = rows
        .iter()
        .filter(|r| mode == 1 && r[2] > 1_000_000 && r[5] * 10 < r[2] && r[12] == 0)
        .count();
    let metrics = host.metrics();
    let parks: u64 = (0..metrics.num_workers())
        .map(|i| metrics.worker_park_count(i))
        .sum();
    println!(
        "UTF8-SCHED-AUDIT-COUNTERS corpus={name} jobs={jobs} mode={mode} handoff_p95_ns={} handoff_p99_ns={} handoff_max_ns={} owner_cpu_p95_ns={} owner_cpu_p99_ns={} owner_cpu_total_ns={} gc_wall_total_ns={} process_switches_in_service_windows={} process_involuntary_in_service_windows={} reply_polls={} reply_pending={} forwarded_reply_wakes={} gc_count={} offcpu_service_spikes={gap} process_cpu_ns={} process_voluntary_switches={} process_involuntary_switches={} process_peak_rss_bytes={} global_queue_depth={} cumulative_worker_parks={parks} diagnostic_only=true",
        latency::percentile(&handoff, 95),
        latency::percentile(&handoff, 99),
        handoff.last().unwrap(),
        latency::percentile(&owner_cpu, 95),
        latency::percentile(&owner_cpu, 99),
        totals[0],
        totals[1],
        totals[2],
        totals[3],
        totals[4],
        totals[5],
        totals[6],
        totals[7],
        after[0] - before[0],
        after[2] - before[2],
        after[3] - before[3],
        after[1],
        metrics.global_queue_depth()
    );
}
