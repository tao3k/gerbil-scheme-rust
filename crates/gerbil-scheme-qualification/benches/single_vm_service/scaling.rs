//! Same short-CPU corpus, fixed owner assignment, one live request per owner.
use super::{
    Arm, GATE, InstanceDispatcher, Partition, Progress, Sample, input_corpus, service_partition,
    snapshot_on,
};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Barrier, oneshot};

const TOTAL: usize = 1_000_000;

struct OwnerSample {
    owner: usize,
    sample: Sample,
}
struct Cohort {
    samples: Vec<OwnerSample>,
    released: Instant,
    elapsed: Duration,
}

async fn cohort(
    dispatch: &InstanceDispatcher,
    progress: &Progress,
    owners: usize,
    messages: usize,
    inputs: &Arc<Vec<Vec<Vec<i64>>>>,
) -> Cohort {
    let barrier = Arc::new(Barrier::new(owners + 1));
    let mut ready = Vec::with_capacity(owners);
    let mut tasks = Vec::with_capacity(owners);
    let mut assigned = 0;
    for owner in 0..owners {
        let count = messages / owners + usize::from(owner < messages % owners);
        assert!(count > 0);
        assert!(owner + (count - 1) * owners < messages);
        assigned += count;
        let (send, receive) = oneshot::channel();
        ready.push(receive);
        let (dispatch, progress, inputs, barrier) = (
            dispatch.clone(),
            progress.clone(),
            Arc::clone(inputs),
            Arc::clone(&barrier),
        );
        tasks.push(tokio::spawn(async move {
            send.send(()).unwrap();
            barrier.wait().await;
            let sample = service_partition(
                dispatch,
                inputs,
                true,
                progress,
                Partition {
                    owner,
                    owners,
                    messages: count,
                },
            )
            .await;
            assert_eq!(sample.latency.len(), count);
            OwnerSample { owner, sample }
        }));
    }
    assert_eq!(assigned, messages);
    for ready in ready {
        ready.await.unwrap();
    }
    let released = Instant::now();
    barrier.wait().await;
    let mut samples = Vec::with_capacity(owners);
    for task in tasks {
        samples.push(task.await.unwrap());
    }
    let last = samples.iter().map(|s| end(&s.sample)).max().unwrap();
    Cohort {
        samples,
        released,
        elapsed: last.duration_since(released),
    }
}

fn end(sample: &Sample) -> Instant {
    sample.started + Duration::from_nanos(u64::try_from(sample.elapsed_ns).unwrap())
}

fn absorb(all: &mut Sample, mut sample: Sample) {
    all.latency.append(&mut sample.latency);
    all.entry.append(&mut sample.entry);
    all.construct.append(&mut sample.construct);
    all.service.append(&mut sample.service);
    all.projection.append(&mut sample.projection);
    all.recovery.append(&mut sample.recovery);
    all.oracle_ns += sample.oracle_ns;
    all.rows += sample.rows;
}

pub(super) fn run(
    dispatch: &InstanceDispatcher,
    runtime: &tokio::runtime::Runtime,
    progress: &Progress,
    owners: usize,
) {
    assert!([1, 12].contains(&owners));
    let inputs = input_corpus(2048, true, false);
    for warmup in [true, false] {
        let messages = if warmup { owners * 32 } else { TOTAL };
        let before: Vec<_> = (0..owners)
            .map(|owner| snapshot_on(dispatch, owner))
            .collect();
        let cohort = runtime.block_on(cohort(dispatch, progress, owners, messages, &inputs));
        let after: Vec<_> = (0..owners)
            .map(|owner| snapshot_on(dispatch, owner))
            .collect();
        let mut all = Sample::new();
        let mut first = cohort.elapsed.as_nanos();
        let mut last = 0;
        for mut owner in cohort.samples {
            let completed = end(&owner.sample)
                .duration_since(cohort.released)
                .as_nanos();
            first = first.min(completed);
            last = last.max(completed);
            let at = owner.owner;
            assert_eq!(after[at].0[3].to_bits(), 0.0_f64.to_bits());
            let mut row = owner
                .sample
                .report(Arm::WorkerService, "short_cpu", 0, warmup);
            row["event"] = json!("fixed_owner_scaling_owner");
            row["owner"] = json!(at);
            row["completed_offset_ns"] = json!(completed);
            row["started_offset_ns"] = json!(
                owner
                    .sample
                    .started
                    .duration_since(cohort.released)
                    .as_nanos()
            );
            row["gc_collections"] = json!(after[at].0[0] - before[at].0[0]);
            row["heap_bytes_before"] = json!(before[at].0[1]);
            row["heap_bytes_after"] = json!(after[at].0[1]);
            row["roots_after"] = json!(after[at].0[3]);
            row["diagnostic_delta"] = json!(
                before[at]
                    .1
                    .zip(after[at].1)
                    .map(|(a, b)| std::array::from_fn::<_, 6, _>(|i| b[i] - a[i]))
            );
            row["diagnostic_order"] = json!([
                "gc_user_s",
                "gc_system_s",
                "gc_wall_s",
                "owner_thread_cpu_s",
                "roots",
                "gc_count"
            ]);
            println!("{row}");
            absorb(&mut all, owner.sample);
        }
        assert_eq!(all.latency.len(), messages);
        all.elapsed_ns = cohort.elapsed.as_nanos();
        let mut row = all.report(Arm::WorkerService, "short_cpu", 0, warmup);
        row["event"] = json!("fixed_owner_scaling_result");
        row["owners"] = json!(owners);
        row["outstanding_per_owner"] = json!(1);
        row["owner_completion_spread_ns"] = json!(last - first);
        row["latency_population"] =
            json!("all completed messages pooled; not owner percentile averages");
        row["throughput_scope"] =
            json!("barrier release to last client completion including oracle");
        row["assignment"] = json!("fixed strided partition; exactly one million total messages");
        row["count_balance_is_constructed"] = json!(true);
        row["production_open_arrival_acceptance"] = json!(false);
        println!("{row}");
    }
    let deadline = Instant::now() + GATE;
    while dispatch
        .owner_capacity()
        .iter()
        .any(|c| c.available != c.limit || c.work_available != c.work_limit)
    {
        assert!(Instant::now() < deadline, "scaling credits did not retire");
        std::thread::yield_now();
    }
}
