//! Same fixed-owner input partition, native work and FIFO result positions.
use super::{
    GATE,
    workload::{Reply, ShortMode, orders},
};
use gerbil_scheme::instances::{InstanceDispatcher, SchemeServiceStreamConfig};
use gerbil_scheme_python::owner_dispatch::OwnerResult;
use std::{collections::VecDeque, sync::Arc, time::Instant};
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    released: Instant,
) -> Vec<OwnerResult<Reply>> {
    let count = inputs.len() / 8;
    let mut groups: [VecDeque<_>; 4] = std::array::from_fn(|_| VecDeque::new());
    for (index, items) in inputs.chunks(8).enumerate() {
        groups[index / 4 % 4].push_back((index, items.to_vec()));
    }
    let mut tasks = tokio::task::JoinSet::new();
    for (owner, mut group) in groups.into_iter().enumerate() {
        let dispatch = dispatch.clone();
        tasks.spawn(async move {
            let mut results = Vec::new();
            while !group.is_empty() {
                // Four owners stage at most 128 original callbacks in aggregate,
                // equal to the host driver's existing 128-ticket window ceiling.
                let chunk: Vec<_> = group.drain(..group.len().min(32)).collect();
                let (indices, messages): (Vec<_>, Vec<_>) = chunk
                    .into_iter()
                    .map(|(index, items)| (index, orders(items, ShortMode::Batch, None)))
                    .unzip();
                let mut stream = dispatch
                    .submit_scheme_service_stream_on(
                        messages,
                        SchemeServiceStreamConfig {
                            owner,
                            maximum_active: 3,
                            maximum_staged_work: 256,
                            timeout: GATE.saturating_sub(released.elapsed()),
                        },
                    )
                    .await
                    .unwrap();
                for index in indices {
                    results.push((
                        index,
                        stream
                            .next(GATE.saturating_sub(released.elapsed()))
                            .await
                            .unwrap(),
                    ));
                }
            }
            results
        });
    }
    let mut results: Vec<_> = (0..count).map(|_| None).collect();
    while let Some(group) = tasks.join_next().await {
        for (index, value) in group.unwrap() {
            assert!(results[index].replace(value).is_none());
        }
    }
    results.into_iter().map(Option::unwrap).collect()
}
