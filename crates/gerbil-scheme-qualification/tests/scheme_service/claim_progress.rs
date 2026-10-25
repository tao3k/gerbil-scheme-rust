//! Unified claim/final waiting through actual fixed-owner Scheme service APIs.
use super::{GATE, HeldProjection, block, corpus, idle, probe, short, unblock};
use gerbil_scheme::instances::{
    InstanceArrival, InstanceDispatcher, InstanceError, SchemeShortPool,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
pub(super) async fn contract(dispatch: &InstanceDispatcher, source: &SchemeShortPool) {
    preclaim(dispatch, source).await;
    claimed_timeout(dispatch, source).await;
    observed_result(dispatch, source).await;
}
async fn preclaim(dispatch: &InstanceDispatcher, source: &SchemeShortPool) {
    let blocked: Vec<_> = (0..4).map(|owner| block(dispatch, owner)).collect();
    let before = dispatch.owner_capacity();
    let ticket = source.submit_progress(probe(), GATE).await.unwrap();
    assert!(matches!(
        source
            .submit_progress(probe(), Duration::from_millis(1))
            .await,
        Err(InstanceError::Timeout)
    ));
    assert_eq!(
        before.iter().map(|c| c.available).collect::<Vec<_>>(),
        dispatch
            .owner_capacity()
            .iter()
            .map(|c| c.available)
            .collect::<Vec<_>>()
    );
    drop(ticket);
    let cancelled = source.submit_progress(probe(), GATE).await.unwrap();
    assert!(matches!(
        cancelled.wait_completed(Duration::ZERO).await,
        Err(InstanceError::Timeout)
    ));
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let expired = source
        .submit_progress(probe(), Duration::from_millis(1))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(2)).await;
    unblock(blocked);
    let observed = expired.wait_observed_ready(issue, GATE).await.unwrap();
    assert!(matches!(
        observed.observed.result,
        Err(InstanceError::Timeout)
    ));
    assert!(observed.observed.progress.is_ok());
    idle(dispatch).await;
}
async fn claimed_timeout(dispatch: &InstanceDispatcher, source: &SchemeShortPool) {
    let (entered, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let ticket = source
        .submit_progress(
            HeldProjection {
                entered,
                release: wait,
            },
            GATE,
        )
        .await
        .unwrap();
    let owner = ready.recv_timeout(GATE).unwrap();
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let mut driver = gerbil_scheme::instances::InstanceClientProgressDriver::new(
        ticket.wait_observed_ready(issue, Duration::from_millis(1)),
    );
    driver
        .wait(tokio::time::sleep(Duration::from_millis(2)))
        .await;
    assert!(matches!(driver.finish().await, Err(InstanceError::Timeout)));
    assert_eq!(
        dispatch.owner_capacity()[owner].available,
        3,
        "claim notification must not free native ownership"
    );
    release.send(()).unwrap();
    idle(dispatch).await;
}
async fn observed_result(dispatch: &InstanceDispatcher, source: &SchemeShortPool) {
    let mut input = corpus::order(1007, false);
    input.truncate(14);
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let result = source
        .submit_progress(short::Short(input.clone()), GATE)
        .await
        .unwrap()
        .wait_observed_ready(issue, GATE)
        .await
        .unwrap();
    corpus::verify(&input, &result.observed.result.unwrap()[0]);
    let timing = result.observed.progress.unwrap();
    if let Some(at) = result.claim.observed {
        assert!(at <= timing.client_observed);
        assert!(result.claim.owner.is_some_and(|owner| owner < 4));
    }
    if result.claim.final_without_claim {
        assert!(result.claim.observed.is_none());
    }
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .issue()
        .unwrap();
    let failed = source
        .submit_progress(short::Short(corpus::order(1008, false)), GATE)
        .await
        .unwrap()
        .wait_observed_ready(issue, GATE)
        .await
        .unwrap();
    assert!(matches!(
        failed.observed.result,
        Err(InstanceError::Scheme(_))
    ));
    assert!(failed.observed.progress.is_ok());
    idle(dispatch).await;
}
