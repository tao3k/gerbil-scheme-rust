//! Progress hints cannot return capacity or accept malformed native metadata.
use crate::instances::service_progress::Progress;
use std::sync::atomic::Ordering;
#[test]
fn receipt_is_coherent_and_retains_completed_ids() {
    let p = Progress::default();
    p.enabled.store(true, Ordering::Release);
    p.charge(8, false);
    p.charge(1, true);
    assert_eq!(p.load(), 9);
    let mut values = vec![-6, 3, 1, 7, 9];
    p.receipt(&mut values).unwrap();
    assert_eq!(values, [7, 9]);
    assert_eq!(
        (p.snapshot().bulk_remaining, p.snapshot().short_remaining),
        (3, 1)
    );
    assert!(p.receipt(&mut vec![-6, -1, 0]).is_err());
    assert!(p.receipt(&mut vec![7]).is_err());
    assert_eq!(p.load(), 4);
}
#[test]
fn disabled_progress_keeps_original_wire_protocol() {
    let p = Progress::default();
    p.charge(8, false);
    let mut values = vec![7, 9];
    p.receipt(&mut values).unwrap();
    assert_eq!(values, [7, 9]);
    assert_eq!(p.load(), 0);
}
