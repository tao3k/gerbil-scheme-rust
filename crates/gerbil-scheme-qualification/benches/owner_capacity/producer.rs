//! Same input batches, staged in bounded per-owner cursors instead of per-turn wakeups.
use super::GATE;
use gerbil_scheme::instances::{BatchAdmission, InstanceDispatcher, ProfiledBatch};
use gerbil_scheme_python::{
    owner_dispatch::{OwnerResult, WindowBudget},
    owner_flow::{FlowControl, owner_flow_backpressured},
};
use std::time::Instant;
type Job = (usize, Vec<Vec<i64>>);
pub(super) struct Produced {
    pub rows: Vec<OwnerResult<ProfiledBatch<i64>>>,
    pub offered: Vec<Instant>,
}
pub(super) async fn run(
    dispatch: InstanceDispatcher,
    jobs: Vec<Job>,
    released: Instant,
) -> Produced {
    let count = jobs.len();
    let mut owners: [Vec<(usize, Vec<Vec<i64>>)>; 4] = std::array::from_fn(|_| Vec::new());
    for (index, (owner, items)) in jobs.into_iter().enumerate() {
        owners[owner].push((index, items));
    }
    let mut indices = Vec::new();
    let mut staged = Vec::new();
    for (owner, inputs) in owners.into_iter().enumerate() {
        let mut turns = inputs.into_iter();
        loop {
            let chunk: Vec<_> = turns.by_ref().take(128).collect();
            if chunk.is_empty() {
                break;
            }
            let (positions, batches): (Vec<_>, Vec<_>) = chunk.into_iter().unzip();
            indices.push(positions);
            staged.push((owner, batches));
        }
    }
    let rows = owner_flow_backpressured(
        staged,
        FlowControl {
            budget: WindowBudget {
                released,
                timeout: GATE,
            },
            maximum_in_flight: 128,
        },
        move |owner, batches| {
            let dispatch = dispatch.clone();
            async move {
                dispatch
                    .submit_numeric_batch_stream_profiled_backpressured(
                        batches,
                        BatchAdmission {
                            owner,
                            yield_every: 1,
                            timeout: GATE.saturating_sub(released.elapsed()),
                        },
                    )
                    .await
            }
        },
    )
    .await;
    let mut outputs: Vec<Option<OwnerResult<ProfiledBatch<i64>>>> =
        (0..count).map(|_| None).collect();
    let mut offered = vec![None; count];
    for (positions, row) in indices.into_iter().zip(rows) {
        let (batches, _) = row.expect("native producer must complete");
        assert_eq!(positions.len(), batches.len());
        for (index, batch) in positions.into_iter().zip(batches) {
            offered[index] = Some(batch.offered);
            outputs[index] = Some(
                batch
                    .value
                    .map(|value| (value, batch.completed.expect("executed turn completion"))),
            );
        }
    }
    Produced {
        rows: outputs.into_iter().map(Option::unwrap).collect(),
        offered: offered.into_iter().map(Option::unwrap).collect(),
    }
}
