//! Actual native-ready and client-consumed events are independent of capacity.
use super::{GATE, batch, idle};
use gerbil_scheme::instances::{InstanceArrival, InstanceDispatcher, InstanceError};
use std::time::{Duration, Instant};
pub(super) async fn contract(
    dispatch: &InstanceDispatcher,
    progress: &std::sync::mpsc::Sender<&'static str>,
) {
    idle(dispatch).await;
    let previous = Instant::now();
    let planned = previous.checked_sub(Duration::from_millis(5)).unwrap();
    let issue = InstanceArrival::new(planned, Some(previous))
        .unwrap()
        .wait_until_due()
        .await
        .unwrap();
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, batch(false), GATE)
        .await
        .unwrap();
    // Let native construction/execution/projection and retirement finish before
    // this client receives the already-ready result. No timing threshold is used.
    idle(dispatch).await;
    let observed = ticket.wait_observed(issue, GATE).await.unwrap();
    let value = observed.result.unwrap();
    let p = observed.progress.unwrap();
    assert_eq!(value.len(), 8);
    assert_eq!(p.timer_armed, Some(false));
    assert!(p.serial_backlog() >= Duration::from_millis(5));
    assert!(p.issued <= p.native_completed && p.native_completed <= p.client_observed);
    assert_eq!(
        p.client_observed.duration_since(p.planned),
        p.serial_backlog()
            + p.before_wait()
            + p.deadline_resume()
            + p.native_completed.duration_since(p.issued)
            + p.delivery()
    );
    let planned = Instant::now() + Duration::from_secs(100);
    let arrival = InstanceArrival::new(planned, Some(p.client_observed)).unwrap();
    assert!(arrival.issue().is_err());
    let issue = InstanceArrival::new(
        Instant::now() + Duration::from_millis(2),
        Some(p.client_observed),
    )
    .unwrap()
    .wait_until_due()
    .await
    .unwrap();
    let ticket = dispatch
        .submit_native_async_on::<(), _>(0, |_| {
            Err(InstanceError::InvalidConfig(
                "injected client progress error",
            ))
        })
        .unwrap();
    let observed = ticket.wait_observed(issue, GATE).await.unwrap();
    assert!(matches!(
        observed.result,
        Err(InstanceError::InvalidConfig(
            "injected client progress error"
        ))
    ));
    let p = observed.progress.unwrap();
    assert!(p.native_completed <= p.client_observed);
    super::command(dispatch, 0, 9);
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .wait_until_due()
        .await
        .unwrap();
    let mut blocked = batch(false);
    blocked.items[0][1] = 2;
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, blocked, GATE)
        .await
        .unwrap();
    assert!(matches!(
        ticket.wait_observed(issue, Duration::ZERO).await,
        Err(InstanceError::Timeout)
    ));
    assert!(dispatch.owner_capacity()[0].available < dispatch.owner_capacity()[0].limit);
    super::command(dispatch, 0, 8);
    idle(dispatch).await;
    super::command(dispatch, 0, 9);
    let issue = InstanceArrival::new(Instant::now(), None)
        .unwrap()
        .wait_until_due()
        .await
        .unwrap();
    let ticket = dispatch
        .submit_scheme_service_backpressured_on(0, batch(false), GATE)
        .await
        .unwrap();
    drop(ticket.wait_observed(issue, GATE));
    idle(dispatch).await;
    assert!(
        InstanceArrival::new(
            Instant::now(),
            Some(Instant::now() + Duration::from_secs(1))
        )
        .is_err()
    );
    progress.send("SERVICE-PROGRESS client planned/issued/native-ready/consumed boundaries, late timer bypass, error/timeout and dropped observation drain passed").unwrap();
}
