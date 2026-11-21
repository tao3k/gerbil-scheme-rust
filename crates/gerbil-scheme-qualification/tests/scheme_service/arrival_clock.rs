//! Kernel arrival does not admit native work or bypass bounded short supply.
use super::{GATE, block, idle, probe, unblock};
use gerbil_scheme::instances::{
    InstanceArrival, InstanceArrivalClock, InstanceDispatcher, InstanceError, SchemeShortPool,
};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
pub(super) async fn contract(dispatch: &InstanceDispatcher, source: &SchemeShortPool) {
    idle(dispatch).await;
    let blocked: Vec<_> = (0..4).map(|owner| block(dispatch, owner)).collect();
    let before: Vec<_> = dispatch
        .owner_capacity()
        .iter()
        .map(|c| (c.available, c.work_available))
        .collect();
    let mut clock = InstanceArrivalClock::new().unwrap();
    let arrival = InstanceArrival::new(Instant::now() + Duration::from_secs(1), None).unwrap();
    let mut cancelled = Box::pin(arrival.wait_with_clock(&mut clock));
    assert!(matches!(
        cancelled
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    drop(cancelled);
    let (issue, wake) = InstanceArrival::new(Instant::now() + Duration::from_micros(500), None)
        .unwrap()
        .wait_with_clock(&mut clock)
        .await
        .unwrap();
    assert_eq!(wake.generation, 2);
    assert!(wake.planned <= wake.observed && wake.observed <= issue.issued_at());
    let queued = source.submit_progress(probe(), GATE).await.unwrap();
    assert_eq!(
        before,
        dispatch
            .owner_capacity()
            .iter()
            .map(|c| (c.available, c.work_available))
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        source.submit_progress(probe(), Duration::ZERO).await,
        Err(InstanceError::Timeout)
    ));
    assert!(matches!(
        queued.wait_observed_ready(issue, Duration::ZERO).await,
        Err(InstanceError::Timeout)
    ));
    let (issue, _) = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .wait_with_clock(&mut clock)
        .await
        .unwrap();
    let queued = source.submit_progress(probe(), GATE).await.unwrap();
    unblock(blocked);
    let observed = queued.wait_observed_ready(issue, GATE).await.unwrap();
    assert_eq!(observed.observed.result.unwrap().len(), 1);
    let timing = observed.observed.progress.unwrap();
    assert_eq!(timing.timer_armed, Some(true));
    assert!(
        timing.issued <= timing.native_completed
            && timing.native_completed <= timing.client_observed
    );
    idle(dispatch).await;
}
