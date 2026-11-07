//! Independent admission, executed records and result progress on real Scheme.
use super::*;
pub(super) fn contract(
    runtime: &tokio::runtime::Runtime,
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) {
    assert_eq!(
        dispatch
            .scheme_service_work_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap()
            .quantum,
        0
    );
    assert!(dispatch.configure_scheme_service_work_on(0, 257).is_err());
    assert!(
        dispatch
            .configure_scheme_service_work_on(0, 4)
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    dispatch
        .configure_scheme_service_budget_on(0, 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(
        dispatch
            .configure_scheme_service_work_on(0, 4)
            .unwrap()
            .wait(GATE)
            .unwrap(),
        4
    );
    assert!(
        dispatch
            .configure_scheme_service_budget_on(0, 0)
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    orders_and_failures(runtime, dispatch);
    let w = dispatch
        .scheme_service_work_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(
        (
            w.bulk_work,
            w.short_work,
            w.pending_start,
            w.ready_generation
        ),
        (384, 8, 0, 3)
    );
    no_stale_handoff(runtime, dispatch, w.handoffs);
    // Deterministic two-class contention, exception unwind and continuation reentry.
    let direct = command(dispatch, 0, 24);
    assert_eq!(&direct[..3], &[4, 4, 20]);
    assert!(direct[3] >= 1);
    assert_eq!(direct[4], 0);
    assert_eq!(direct[5], 1);
    assert_eq!(direct[6], 1);
    dispatch
        .configure_scheme_service_work_on(0, 0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    dispatch
        .configure_scheme_service_budget_on(0, 0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    progress.send("executed records differ from retained work; uncontended zero handoffs, contention, exception and continuation passed").unwrap();
}

pub(super) fn enable(dispatch: &InstanceDispatcher, owner: usize) {
    dispatch
        .configure_scheme_service_budget_on(owner, 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
    dispatch
        .configure_scheme_service_actor_turn_on(
            owner,
            gerbil_scheme::instances::SchemeWorkExecutionPolicy {
                contended_work: 4,
                maximum_uninterrupted_work: 48,
            },
        )
        .unwrap()
        .wait(GATE)
        .unwrap();
}

fn orders_and_failures(runtime: &tokio::runtime::Runtime, dispatch: &InstanceDispatcher) {
    runtime.block_on(async {
        idle(dispatch).await;
        let capacity = dispatch.owner_capacity()[0].work_available;
        let reply = dispatch
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        assert_eq!(reply.len(), 8);
        idle(dispatch).await;
        assert_eq!(dispatch.owner_capacity()[0].work_available, capacity);
        let w = dispatch
            .scheme_service_work_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!((w.bulk_work, w.short_work, w.handoffs), (8 * 48, 0, 0));
        for failed in [false, true, false] {
            let mut input = corpus::order(100_200, false);
            input.truncate(14);
            if failed {
                input.clear();
            }
            let result = dispatch
                .submit_scheme_service_backpressured_on(0, short::Short(input), GATE)
                .await
                .unwrap()
                .wait_completed(GATE)
                .await;
            assert_eq!(result.is_err(), failed);
            idle(dispatch).await;
        }
    });
}

fn no_stale_handoff(runtime: &tokio::runtime::Runtime, dispatch: &InstanceDispatcher, before: u64) {
    // Ready IDs were harvested: the next bulk must not yield for stale readiness.
    runtime.block_on(async {
        dispatch
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap();
        idle(dispatch).await;
    });
    assert_eq!(
        dispatch
            .scheme_service_work_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap()
            .handoffs,
        before
    );
}
