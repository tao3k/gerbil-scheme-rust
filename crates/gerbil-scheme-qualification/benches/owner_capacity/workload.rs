//! Same typed invoice corpus; only explicit native batching and pool capacity vary.
use super::{GATE, corpus};
use gerbil_scheme::instances::{BatchAdmission, InstanceDispatcher, InstanceError};
use gerbil_scheme_python::{
    owner_dispatch::WindowBudget,
    owner_flow::{FlowControl, owner_flow_backpressured},
};
use std::{sync::Arc, time::Instant};
pub(super) struct Bulk {
    pub elapsed: u128,
    pub clocks: Vec<Vec<Instant>>,
    pub actor_peer_min: Option<usize>,
}
pub(super) struct Probe {
    pub native: Vec<u128>,
    pub delivery: Vec<u128>,
    pub admission: Vec<u128>,
    pub wake: Vec<u128>,
    pub clocks: Vec<Instant>,
}
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    batch: usize,
    released: Instant,
) -> Bulk {
    let jobs = inputs
        .chunks(batch)
        .enumerate()
        .map(|(index, items)| (index * batch / 32 % 4, items.to_vec()))
        .collect();
    let outputs = owner_flow_backpressured(
        jobs,
        FlowControl {
            budget: WindowBudget {
                released,
                timeout: GATE,
            },
            maximum_in_flight: 128,
        },
        move |owner, items| {
            let dispatch = dispatch.clone();
            async move {
                dispatch
                    .submit_numeric_batch_backpressured(
                        items,
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
    let mut clocks = vec![Vec::new(); 4];
    let mut actor_peer_min = None;
    for (chunk, row) in outputs.into_iter().enumerate() {
        let (values, completed) = row.expect("native bulk request must complete");
        assert_eq!(values.len(), batch);
        for (offset, value) in values.iter().enumerate() {
            corpus::verify(&inputs[chunk * batch + offset], value);
            clocks[chunk * batch / 32 % 4].push(completed);
        }
        if batch >= 4 {
            let peers = corpus::peers(&values);
            actor_peer_min = Some(actor_peer_min.map_or(peers, |old: usize| old.min(peers)));
        }
    }
    Bulk {
        elapsed: released.elapsed().as_nanos(),
        clocks,
        actor_peer_min,
    }
}
pub(super) async fn probes(
    dispatch: InstanceDispatcher,
    count: usize,
    released: Instant,
) -> Result<Probe, InstanceError> {
    let mut probe = Probe {
        native: Vec::new(),
        delivery: Vec::new(),
        admission: Vec::new(),
        wake: Vec::new(),
        clocks: Vec::new(),
    };
    for id in 0..count {
        let scheduled =
            released + std::time::Duration::from_micros(u64::try_from(id * 500).unwrap());
        tokio::time::sleep_until(tokio::time::Instant::from_std(scheduled)).await;
        let entered = Instant::now();
        let mut input = corpus::order(100_000 + id, false);
        input.truncate(14); // Four lines, validated by the shared independent oracle.
        let ticket = dispatch
            .submit_numeric_batch_backpressured(
                vec![input.clone()],
                BatchAdmission {
                    owner: id % 4,
                    yield_every: 1,
                    timeout: GATE.saturating_sub(released.elapsed()),
                },
            )
            .await?;
        let admitted = Instant::now();
        let (output, completed) = ticket
            .wait_completed(GATE.saturating_sub(released.elapsed()))
            .await?;
        let delivered = Instant::now();
        corpus::verify(&input, &output[0]);
        probe
            .native
            .push(completed.saturating_duration_since(scheduled).as_nanos());
        probe
            .delivery
            .push(delivered.saturating_duration_since(scheduled).as_nanos());
        probe
            .admission
            .push(admitted.duration_since(entered).as_nanos());
        probe
            .wake
            .push(entered.saturating_duration_since(scheduled).as_nanos());
        probe.clocks.push(completed);
    }
    Ok(probe)
}
