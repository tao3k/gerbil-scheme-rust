//! Existing public routing API: bounded Actor supply and projection-time credits.
use super::*;
use gerbil_scheme::instances::{SchemeRoutingConfig, SchemeServiceRouter};
use std::{future::Future, task::Poll};

fn router(dispatch: &InstanceDispatcher, slots: usize) -> SchemeServiceRouter {
    SchemeServiceRouter::new(
        dispatch.clone(),
        SchemeRoutingConfig {
            owners: vec![0],
            short_slots: slots,
            batch_slots: 4 - slots,
            short_max_work: 1,
            batch_max_work: 8,
        },
    )
    .unwrap()
}
fn input(id: usize) -> Vec<i64> {
    let mut value = corpus::order(id, false);
    value.truncate(14);
    value
}
struct BadConstruction;
impl SchemeServiceRequest for BadConstruction {
    type Output = ();
    fn work_units(&self) -> usize {
        1
    }
    fn short_service(&self) -> bool {
        true
    }
    fn construct<'vm>(&mut self, _: &NativeSession<'vm>) -> Result<NativeRoot<'vm>, InstanceError> {
        Err(InstanceError::InvalidConfig(
            "injected routed construction failure",
        ))
    }
    fn project(self, _: &NativeSession<'_>, _: &NativeRoot<'_>) -> Result<(), InstanceError> {
        panic!("failed construction must not project")
    }
}
#[test]
fn class_credit_returns_at_projection_before_client_consumption_and_after_failure() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    let router = router(&dispatch, 1);
    let first_input = input(71);
    let first = runtime
        .block_on(router.submit_short(short::Short(first_input.clone()), GATE))
        .unwrap();
    runtime.block_on(async {
        let started = Instant::now();
        while router.loads()[0].short_requests != 0 {
            assert!(
                started.elapsed() < GATE,
                "unconsumed result retained route credit"
            );
            tokio::task::yield_now().await;
        }
        let second_input = input(72);
        let second = router
            .submit_short(short::Short(second_input.clone()), GATE)
            .await
            .unwrap();
        let value = second.ticket.wait_completed(GATE).await.unwrap().0;
        corpus::verify(&second_input, &value[0]);
        // First result is deliberately still unconsumed when second completes.
        let value = first.ticket.wait_completed(GATE).await.unwrap().0;
        corpus::verify(&first_input, &value[0]);
        let failed = router.submit_short(BadConstruction, GATE).await.unwrap();
        assert!(matches!(
            failed.ticket.wait_completed(GATE).await,
            Err(InstanceError::InvalidConfig(_))
        ));
        let third_input = input(73);
        let third = router
            .submit_short(short::Short(third_input.clone()), GATE)
            .await
            .unwrap();
        corpus::verify(
            &third_input,
            &third.ticket.wait_completed(GATE).await.unwrap().0[0],
        );
        idle(&dispatch).await;
    });
    assert_eq!(router.loads()[0].short_requests, 0);
    router.close();
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
}
#[test]
fn three_short_jobs_are_admitted_and_fourth_holds_no_native_credit() {
    let _case = super::native_case();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut settings = config();
    settings.owners = 1;
    let mut pool = InstancePool::start(&descriptor(false), settings).unwrap();
    let dispatch = pool.dispatcher();
    dispatch.configure_owner_budgets(4, Some(32)).unwrap();
    let router = router(&dispatch, 3);
    let (entered, running) = mpsc::sync_channel(1);
    let (release, held) = mpsc::sync_channel(1);
    let blocker = dispatch
        .submit_native_on(0, move |_| {
            entered.send(()).unwrap();
            held.recv_timeout(GATE).unwrap();
            Ok(())
        })
        .unwrap();
    running.recv_timeout(GATE).unwrap();
    let mut tickets = Vec::new();
    for id in 81..84 {
        let expected = input(id);
        let ticket = runtime
            .block_on(router.submit_short(short::Short(expected.clone()), GATE))
            .unwrap();
        tickets.push((expected, ticket));
    }
    assert_eq!(dispatch.owner_capacity()[0].available, 0);
    assert_eq!(dispatch.owner_capacity()[0].work_available, Some(28));
    let mut fourth = Box::pin(router.submit_short(short::Short(input(84)), GATE));
    runtime.block_on(std::future::poll_fn(|cx| {
        assert!(fourth.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    }));
    // If it passed the class gate it would take a work credit before waiting
    // for a native slot. Remaining at 28 proves class admission is bounded.
    assert_eq!(dispatch.owner_capacity()[0].work_available, Some(28));
    drop(fourth);
    assert_eq!(router.loads()[0].short_requests, 3);
    release.send(()).unwrap();
    blocker.wait(GATE).unwrap();
    runtime.block_on(async {
        for (expected, ticket) in tickets {
            corpus::verify(
                &expected,
                &ticket.ticket.wait_completed(GATE).await.unwrap().0[0],
            );
        }
        idle(&dispatch).await;
    });
    assert_eq!(router.loads()[0].short_requests, 0);
    assert_eq!(dispatch.owner_capacity()[0].available, 4);
    router.close();
    let receipt = pool.shutdown(GATE);
    assert!(receipt.is_clean(), "{receipt:?}");
}
