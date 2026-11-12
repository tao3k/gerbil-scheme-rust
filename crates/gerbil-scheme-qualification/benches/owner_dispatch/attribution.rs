//! Diagnostic clocks only; normal scheduler samples do not enter this module.
use super::{
    GATE,
    executor::{Completed, Engine, Executor, Job},
};
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError, ProfiledBatch};
use serde_json::json;
use std::time::Instant;
pub(super) struct Frame {
    pub(super) owner: usize,
    pub(super) worker: Instant,
    pub(super) admitted: Instant,
    pub(super) native: Instant,
    pub(super) received: Instant,
    pub(super) collected: Instant,
    pub(super) output: ProfiledBatch<i64>,
}
pub(super) fn window(
    executor: &Executor,
    dispatch: &InstanceDispatcher,
    jobs: Vec<Job>,
    released: Instant,
) -> Vec<Completed> {
    let ready = Instant::now();
    if executor.name == "owner-flow" {
        return super::flow_attribution::window(executor, dispatch, jobs, released, ready);
    }
    if executor.grouped && executor.name != "direct" {
        return super::grouped_attribution::window(executor, dispatch, jobs, released, ready);
    }
    let submit = |(owner, items)| {
        let worker = Instant::now();
        let ticket = if released.elapsed() >= GATE {
            Err(InstanceError::Timeout)
        } else {
            dispatch.submit_numeric_batch_profiled_on(owner, items, 1)
        };
        (owner, worker, Instant::now(), ticket)
    };
    let frames = match &executor.engine {
        Engine::Tokio(runtime) => runtime.block_on(async {
            let tasks: Vec<_> = jobs
                .into_iter()
                .map(|(owner, items)| {
                    let dispatch = dispatch.clone();
                    runtime.spawn(async move {
                        let worker = Instant::now();
                        if released.elapsed() >= GATE {
                            return Err(InstanceError::Timeout);
                        }
                        let ticket =
                            dispatch.submit_numeric_batch_profiled_async_on(owner, items, 1)?;
                        let admitted = Instant::now();
                        let (output, native) = ticket
                            .wait_completed(GATE.saturating_sub(released.elapsed()))
                            .await?;
                        let received = Instant::now();
                        Ok(Frame {
                            owner,
                            worker,
                            admitted,
                            native,
                            received,
                            collected: received,
                            output,
                        })
                    })
                })
                .collect();
            let mut results = Vec::new();
            for task in tasks {
                let mut result = task.await.unwrap_or(Err(InstanceError::TaskPanicked));
                if let Ok(frame) = &mut result {
                    frame.collected = Instant::now();
                }
                results.push(result);
            }
            results
        }),
        engine => {
            let tickets: Vec<_> = if let Engine::Rayon(pool) = engine {
                use rayon::iter::{IntoParallelIterator, ParallelIterator};
                pool.install(|| jobs.into_par_iter().map(submit).collect())
            } else {
                jobs.into_iter().map(submit).collect()
            };
            tickets
                .into_iter()
                .map(|(owner, worker, admitted, ticket)| {
                    let (output, native) =
                        ticket?.wait_completed(GATE.saturating_sub(released.elapsed()))?;
                    let received = Instant::now();
                    Ok(Frame {
                        owner,
                        worker,
                        admitted,
                        native,
                        received,
                        collected: received,
                        output,
                    })
                })
                .collect()
        }
    };
    emit(frames, executor, released, ready)
}
pub(super) fn emit(
    frames: Vec<Result<Frame, InstanceError>>,
    executor: &Executor,
    released: Instant,
    ready: Instant,
) -> Vec<Completed> {
    let mut rows = Vec::new();
    let mut failed = 0;
    let results = frames
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result
                .map(|f| {
                    let t = f.output.timeline;
                    let marks = [
                        released,
                        ready,
                        f.worker,
                        t.dispatched,
                        t.entered,
                        t.constructed,
                        t.executed,
                        t.projected,
                        f.native,
                        f.received,
                        f.collected,
                    ];
                    assert!(marks.windows(2).all(|pair| pair[0] <= pair[1]));
                    let spans: Vec<_> = marks
                        .windows(2)
                        .map(|p| p[1].duration_since(p[0]).as_nanos())
                        .collect();
                    assert_eq!(
                        spans.iter().sum::<u128>(),
                        f.collected.duration_since(released).as_nanos()
                    );
                    rows.push(json!({"index": index, "owner": f.owner, "ns": spans,
            "native_completed_ns": f.native.duration_since(released).as_nanos(),
            "observed_ns": f.received.duration_since(released).as_nanos(),
            "collected_ns": f.collected.duration_since(released).as_nanos(),
            "collections": t.collections,
            "admission_return_ns": f.admitted.duration_since(t.dispatched).as_nanos(),
            "owner_wait_lower_ns": t.entered.saturating_duration_since(f.admitted).as_nanos(),
            "sdk": t.diagnostics}));
                    (f.output.values, f.native)
                })
                .inspect_err(|_| {
                    failed += 1;
                })
        })
        .collect();
    println!(
        "{}",
        json!({"event": "dispatch_attribution", "grouped": executor.grouped, "mode": executor.name, "workers": executor.workers,
        "phase_order": ["input_copy_and_entry", "dispatch_ready_to_worker", "validation", "admission_and_owner_wait", "native_types", "scheme_including_gc", "native_projection", "root_retirement", "result_observation", "ordered_task_collection"], "rows": rows, "failed": failed})
    );
    results
}
