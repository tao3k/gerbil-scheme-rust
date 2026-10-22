//! Same input/FIFO results and per-owner active bounds; no fixed input partition.
use super::{
    GATE,
    workload::{Reply, ShortMode, orders},
};
use gerbil_scheme::instances::{InstanceDispatcher, SchemeServicePoolStreamConfig};
use gerbil_scheme_python::owner_dispatch::OwnerResult;
use std::{sync::Arc, time::Instant};
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    released: Instant,
) -> Vec<OwnerResult<Reply>> {
    let mut results = Vec::new();
    // Same aggregate 128-callback count and 1024 declared-work staging bounds
    // as four fixed sources of 32 callbacks, with three active jobs per owner.
    for chunk in inputs.chunks(128 * 8) {
        let messages = chunk
            .chunks(8)
            .map(|items| orders(items.to_vec(), ShortMode::Batch, None))
            .collect();
        let mut stream = dispatch
            .submit_scheme_service_pool_stream(
                messages,
                SchemeServicePoolStreamConfig {
                    owners: vec![0, 1, 2, 3],
                    maximum_active_per_owner: 3,
                    maximum_staged_work: 1024,
                    timeout: GATE.saturating_sub(released.elapsed()),
                },
            )
            .await
            .unwrap();
        while let Some(row) = stream.next(GATE.saturating_sub(released.elapsed())).await {
            results.push(row);
        }
    }
    results
}
