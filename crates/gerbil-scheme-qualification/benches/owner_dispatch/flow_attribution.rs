//! Diagnostic boundaries for the same completion-driven owner-flow API.
use super::{
    GATE,
    attribution::{Frame, emit},
    executor::{Completed, Engine, Executor, Job},
};
use gerbil_scheme::instances::InstanceDispatcher;
use gerbil_scheme_python::{
    owner_dispatch::WindowBudget,
    owner_flow::{FlowControl, FlowStage, owner_flow_window_observed},
};
use std::{cell::RefCell, time::Instant};
#[derive(Default)]
struct Points {
    worker: Option<Instant>,
    admitted: Option<Instant>,
    collected: Option<Instant>,
}
pub(super) fn window(
    executor: &Executor,
    dispatch: &InstanceDispatcher,
    jobs: Vec<Job>,
    released: Instant,
    ready: Instant,
) -> Vec<Completed> {
    let Engine::Tokio(runtime) = &executor.engine else {
        unreachable!()
    };
    let owners: Vec<_> = jobs.iter().map(|(owner, _)| *owner).collect();
    let points = RefCell::new(
        (0..jobs.len())
            .map(|_| Points::default())
            .collect::<Vec<_>>(),
    );
    let results = runtime.block_on(owner_flow_window_observed(
        jobs,
        FlowControl {
            budget: WindowBudget {
                released,
                timeout: GATE,
            },
            maximum_in_flight: 128,
        },
        |owner, items| dispatch.submit_numeric_batch_profiled_async_on(owner, items, 1),
        |index, stage| {
            let mut points = points.borrow_mut();
            match stage {
                FlowStage::Submitting => points[index].worker = Some(Instant::now()),
                FlowStage::Admitted => points[index].admitted = Some(Instant::now()),
                FlowStage::Collected => points[index].collected = Some(Instant::now()),
            }
        },
    ));
    let points = points.into_inner();
    let frames = results
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result.map(|(output, native)| Frame {
                owner: owners[index],
                worker: points[index].worker.unwrap(),
                admitted: points[index].admitted.unwrap(),
                native,
                received: points[index].collected.unwrap(),
                collected: points[index].collected.unwrap(),
                output,
            })
        })
        .collect();
    emit(frames, executor, released, ready)
}
