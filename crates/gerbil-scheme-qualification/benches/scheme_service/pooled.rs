//! Same input/FIFO results and per-owner active bounds; no fixed input partition.
use super::{
    GATE,
    workload::{Reply, ShortMode, orders},
};
use gerbil_scheme::instances::{
    InstanceDispatcher, SchemeServicePoolStreamConfig, SchemeServiceSupplyAdmission,
};
use gerbil_scheme_python::owner_dispatch::OwnerResult;
use std::{sync::Arc, time::Instant};
pub(super) async fn bulk(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    released: Instant,
    mode: SchemeServiceSupplyAdmission,
    ready: bool,
) -> Vec<OwnerResult<Reply>> {
    let mut results = Vec::new();
    // Same aggregate 128-callback count and 1024 declared-work staging bounds
    // as four fixed sources of 32 callbacks, with three active jobs per owner.
    for chunk in inputs.chunks(128 * 8) {
        let messages = chunk
            .chunks(8)
            .map(|items| orders(items.to_vec(), ShortMode::Batch, None))
            .collect();
        let config = SchemeServicePoolStreamConfig {
            owners: vec![0, 1, 2, 3],
            maximum_active_per_owner: 3,
            maximum_staged_work: 1024,
            timeout: GATE.saturating_sub(released.elapsed()),
        };
        if ready {
            let count = chunk.len().div_ceil(8);
            let mut indexed = (0..count).map(|_| None).collect::<Vec<_>>();
            let mut stream = dispatch
                .submit_scheme_service_pool_ready_stream(messages, config, mode)
                .await
                .unwrap();
            while let Some(row) = stream.next(GATE.saturating_sub(released.elapsed())).await {
                let row = row.unwrap();
                assert!(row.sequence < count && indexed[row.sequence].is_none());
                if let Ok(reply) = &row.result {
                    assert_eq!(Some(reply.owner), row.owner);
                }
                indexed[row.sequence] = Some(row.result.map(|value| (value, row.native_completed)));
            }
            results.extend(indexed.into_iter().map(Option::unwrap));
            continue;
        }
        let mut stream = dispatch
            .submit_scheme_service_pool_stream_with_admission(messages, config, mode)
            .await
            .unwrap();
        while let Some(row) = stream.next(GATE.saturating_sub(released.elapsed())).await {
            results.push(row);
        }
    }
    results
}

pub(super) async fn bulk_progress<P: std::future::Future<Output = super::workload::Probes>>(
    dispatch: InstanceDispatcher,
    inputs: Arc<Vec<Vec<i64>>>,
    released: Instant,
    mode: SchemeServiceSupplyAdmission,
    progress: P,
    bounded_preparation: bool,
    pipelined: bool,
) -> (Vec<OwnerResult<Reply>>, super::workload::Probes) {
    let mut driver = gerbil_scheme::instances::InstanceClientProgressDriver::new(progress);
    let mut results = Vec::new();
    for chunk in inputs.chunks(128 * 8) {
        let messages = chunk
            .chunks(8)
            .map(|items| Ok(orders(items.to_vec(), ShortMode::Batch, None)));
        let config = SchemeServicePoolStreamConfig {
            owners: vec![0, 1, 2, 3],
            maximum_active_per_owner: 3,
            maximum_staged_work: 1024,
            timeout: GATE.saturating_sub(released.elapsed()),
        };
        let count = chunk.len().div_ceil(8);
        let mut indexed = (0..count).map(|_| None).collect::<Vec<_>>();
        let mut stream = if pipelined {
            dispatch
                .pipeline_scheme_service_pool_ready_stream(messages, config, mode, &mut driver)
                .await
        } else if bounded_preparation {
            dispatch
                .prepare_scheme_service_pool_ready_stream(messages, config, mode, &mut driver)
                .await
        } else {
            let messages = messages.collect::<Result<Vec<_>, _>>().unwrap();
            driver
                .wait(dispatch.submit_scheme_service_pool_ready_stream(messages, config, mode))
                .await
        }
        .unwrap();
        while let Some(row) = driver
            .wait(stream.next(GATE.saturating_sub(released.elapsed())))
            .await
        {
            let row = row.unwrap();
            assert!(row.sequence < count && indexed[row.sequence].is_none());
            if let Ok(reply) = &row.result {
                assert_eq!(Some(reply.owner), row.owner);
            }
            indexed[row.sequence] = Some(row.result.map(|value| (value, row.native_completed)));
        }
        results.extend(indexed.into_iter().map(Option::unwrap));
    }
    (results, driver.finish().await)
}
