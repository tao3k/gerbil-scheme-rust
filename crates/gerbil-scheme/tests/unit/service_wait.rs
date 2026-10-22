//! ABI boundary: finite deadlines must never alias Scheme's infinite wait.
use crate::instances::service_loop::wait_timeout;
use std::time::Duration;
#[test]
fn finite_deadlines_never_become_infinite_waits() {
    assert_eq!(wait_timeout(None), 0);
    for finite in [
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_nanos(999),
        Duration::from_micros(1),
        Duration::MAX,
    ] {
        assert!(
            wait_timeout(Some(finite)) > 0,
            "finite {finite:?} became infinite"
        );
    }
    assert_eq!(wait_timeout(Some(Duration::from_nanos(1001))), 2);
    assert_eq!(wait_timeout(Some(Duration::MAX)), i64::MAX);
}
