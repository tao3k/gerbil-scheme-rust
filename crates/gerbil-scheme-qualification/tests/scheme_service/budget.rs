//! Actual Scheme unit arbitration, release-on-wait and continuation-safe reuse.
use super::*;
pub(super) fn configure(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    assert!(matches!(
        dispatch.configure_scheme_service_budget_on(0, 257),
        Err(InstanceError::InvalidConfig(_))
    ));
    for owner in 0..4 {
        let before = dispatch
            .scheme_service_budget_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!((before.short_units, before.bulk_units), (0, 0));
        dispatch
            .configure_scheme_service_budget_on(owner, 1)
            .unwrap()
            .wait(GATE)
            .unwrap();
    }
    let contract = command(dispatch, 0, 13);
    assert_eq!(&contract[..4], &[0, 1, 2, 3]);
    assert_eq!(&contract[4..], &[2, 2, 0, 1, 1, 2, 0, 0, 0]);
    progress
        .send("native Scheme unit budget/class debt and two-class contention passed")
        .unwrap();
    let stress = command(dispatch, 0, 14);
    assert_eq!(&stress[..5], &[256, 256, 64, 192, 0]);
    assert!(
        stress[5] <= 1 && stress[6] <= 1,
        "class budget exceeded: {stress:?}"
    );
    assert!(
        stress[7] >= 32,
        "the fairness stress control did not contend"
    );
    assert_eq!(&stress[8..], &[0, 0, 0]);
    progress
        .send("native repeated class contention verified 256 real orders per policy and drained")
        .unwrap();
}

pub(super) async fn toggle_retained(dispatch: &InstanceDispatcher, owner: usize, enabled: bool) {
    idle(dispatch).await;
    dispatch
        .configure_scheme_service_budget_on(owner, usize::from(enabled))
        .unwrap()
        .wait(GATE)
        .unwrap();
}
