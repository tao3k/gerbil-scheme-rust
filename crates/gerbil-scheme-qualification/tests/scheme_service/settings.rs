//! Configuration readback comes from Scheme and survives both wake-mode changes.
use super::{GATE, idle};
use gerbil_scheme::instances::{InstanceDispatcher, SchemeServiceWake};
use std::sync::mpsc;
pub(super) fn defaults(dispatch: &InstanceDispatcher) {
    for owner in 0..4 {
        let actual = dispatch
            .scheme_service_settings_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(actual.cooperative_budget, 0);
        assert_eq!(actual.wake, SchemeServiceWake::Watcher);
    }
}
pub(super) async fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    idle(dispatch).await;
    for owner in 0..4 {
        let saved = dispatch
            .scheme_service_settings_on(owner)
            .unwrap()
            .wait(GATE)
            .unwrap();
        for (budget, wake) in [
            (0, SchemeServiceWake::Watcher),
            (1, SchemeServiceWake::NativePort),
            (256, SchemeServiceWake::Watcher),
        ] {
            dispatch
                .configure_scheme_service_budget_on(owner, budget)
                .unwrap()
                .wait(GATE)
                .unwrap();
            dispatch
                .configure_scheme_service_wake_on(owner, wake)
                .unwrap()
                .wait(GATE)
                .unwrap();
            let actual = dispatch
                .scheme_service_settings_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert_eq!((actual.cooperative_budget, actual.wake), (budget, wake));
        }
        dispatch
            .configure_scheme_service_budget_on(owner, saved.cooperative_budget)
            .unwrap()
            .wait(GATE)
            .unwrap();
        dispatch
            .configure_scheme_service_wake_on(owner, saved.wake)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert_eq!(
            dispatch
                .scheme_service_settings_on(owner)
                .unwrap()
                .wait(GATE)
                .unwrap(),
            saved
        );
    }
    progress.send("native settings: actual budget/wake readback, zero/max values, both modes and restoration").unwrap();
}
