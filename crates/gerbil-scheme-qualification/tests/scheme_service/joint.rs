//! Actual Scheme acknowledgement, exception drain and policy reset.
use super::*;
fn read(dispatch: &InstanceDispatcher) -> gerbil_scheme::instances::SchemeJointProgress {
    dispatch
        .scheme_service_joint_progress_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap()
}
pub(super) fn contract(
    runtime: &tokio::runtime::Runtime,
    dispatch: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) {
    assert!(!read(dispatch).enabled);
    assert!(
        dispatch
            .configure_scheme_service_joint_progress_on(0, true)
            .unwrap()
            .wait(GATE)
            .unwrap()
    );
    runtime.block_on(async {
        for failed in [false, true, false] {
            let mut value = corpus::order(100_000, false);
            if failed {
                value.clear();
            } else {
                value.truncate(14);
            }
            let ticket = dispatch
                .submit_scheme_service_backpressured_on(0, short::Short(value), GATE)
                .await
                .unwrap();
            let result = ticket.wait_completed(GATE).await;
            if failed {
                assert!(matches!(result, Err(InstanceError::Scheme(_))));
            } else {
                assert_eq!(result.unwrap().0.len(), 1);
            }
            idle(dispatch).await;
        }
    });
    let actual = read(dispatch);
    assert!(actual.enabled);
    assert_eq!(actual.pending_dispatch, 0);
    assert_eq!(actual.worker_starts, 3);
    // Checkpoints are opportunities, not a required timing-dependent count.
    assert!(
        !dispatch
            .configure_scheme_service_joint_progress_on(0, false)
            .unwrap()
            .wait(GATE)
            .unwrap()
    );
    assert_eq!(read(dispatch).worker_starts, 0);
    progress
        .send("joint Scheme dispatch acknowledgement, short exceptions and idle disable passed")
        .unwrap();
}

pub(super) fn enable(dispatch: &InstanceDispatcher) {
    assert!(
        dispatch
            .configure_scheme_service_joint_progress_on(0, true)
            .unwrap()
            .wait(GATE)
            .unwrap()
    );
}
pub(super) fn assert_disabled(dispatch: &InstanceDispatcher) {
    assert!(!read(dispatch).enabled);
}
pub(super) fn abandon(runtime: &tokio::runtime::Runtime, dispatch: &InstanceDispatcher) {
    enable(dispatch);
    work::enable(dispatch, 0);
    runtime.block_on(async {
        let mut value = corpus::order(100_001, false);
        value.truncate(14);
        drop(
            dispatch
                .submit_scheme_service_backpressured_on(0, short::Short(value), GATE)
                .await
                .unwrap(),
        );
    });
    // The caller immediately closes this same VM: an unentered request may
    // discard; an entered one must acknowledge and drain before cleanup.
}
