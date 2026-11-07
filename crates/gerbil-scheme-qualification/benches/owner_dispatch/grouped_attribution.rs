//! Candidate diagnostics reuse the actual grouped scheduler and native API.
use super::{
    GATE,
    attribution::{Frame, emit},
    executor::{Completed, Engine, Executor, Job},
};
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError};
use gerbil_scheme_python::owner_dispatch::{
    CollectionStage, WindowBudget, rayon_owner_submit, tokio_owner_window_observed,
};
use std::{
    sync::{Arc, OnceLock},
    time::Instant,
};
#[derive(Default)]
struct Points {
    worker: OnceLock<Instant>,
    admitted: OnceLock<Instant>,
    received: OnceLock<Instant>,
    collected: OnceLock<Instant>,
}
pub(super) fn window(
    executor: &Executor,
    dispatch: &InstanceDispatcher,
    jobs: Vec<Job>,
    released: Instant,
    ready: Instant,
) -> Vec<Completed> {
    let frames = match &executor.engine {
        Engine::Tokio(runtime) => {
            let points: Arc<Vec<Points>> =
                Arc::new((0..jobs.len()).map(|_| Points::default()).collect());
            let admission = Arc::clone(&points);
            let delivery = Arc::clone(&points);
            let owners: Vec<_> = jobs.iter().map(|(owner, _)| *owner).collect();
            let dispatch = dispatch.clone();
            let jobs = jobs
                .into_iter()
                .enumerate()
                .map(|(i, (owner, items))| (owner, (i, items)))
                .collect();
            let results = runtime.block_on(tokio_owner_window_observed(
                jobs,
                WindowBudget {
                    released,
                    timeout: GATE,
                },
                move |owner, (index, items)| {
                    admission[index].worker.set(Instant::now()).unwrap();
                    let ticket =
                        dispatch.submit_numeric_batch_profiled_async_on(owner, items, 1)?;
                    admission[index].admitted.set(Instant::now()).unwrap();
                    Ok(ticket)
                },
                move |index, stage| match stage {
                    CollectionStage::Received => {
                        delivery[index].received.set(Instant::now()).unwrap();
                    }
                    CollectionStage::Collected => {
                        delivery[index].collected.set(Instant::now()).unwrap();
                    }
                },
            ));
            results
                .into_iter()
                .enumerate()
                .map(|(index, result)| {
                    result.map(|(output, native)| Frame {
                        owner: owners[index],
                        worker: *points[index].worker.get().unwrap(),
                        admitted: *points[index].admitted.get().unwrap(),
                        native,
                        received: *points[index].received.get().unwrap(),
                        collected: *points[index].collected.get().unwrap(),
                        output,
                    })
                })
                .collect()
        }
        Engine::Rayon(pool) => {
            let tickets = rayon_owner_submit(pool, jobs, |owner, items| {
                let worker = Instant::now();
                if released.elapsed() >= GATE {
                    return Err(InstanceError::Timeout);
                }
                let ticket = dispatch.submit_numeric_batch_profiled_on(owner, items, 1)?;
                Ok((owner, worker, Instant::now(), ticket))
            });
            tickets
                .into_iter()
                .map(|ticket| {
                    let (owner, worker, admitted, ticket) = ticket?;
                    let (output, native) =
                        ticket.wait_completed(GATE.saturating_sub(released.elapsed()))?;
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
        Engine::Direct => unreachable!("direct A/A uses the unchanged diagnostic window"),
    };
    emit(frames, executor, released, ready)
}
