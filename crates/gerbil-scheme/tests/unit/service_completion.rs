//! Epoch/readiness metadata cannot invent authority over a newer publication.
use super::{SchemeCompletionProgress, SchemeServiceCompletion};
#[test]
fn older_owned_token_can_coexist_with_newer_ready_epoch() {
    let actual = SchemeCompletionProgress::parse(&[2, 2, 1, 1, 1, 1, 2, 1]).unwrap();
    assert_eq!(actual.policy, SchemeServiceCompletion::CoalescedHandoff);
    assert!(actual.token_owned && actual.opportunity_pending);
    assert_eq!(
        (actual.epoch, actual.claimed, actual.ready_queued),
        (2, 1, 1)
    );
    let cycle = SchemeCompletionProgress::parse(&[3, 2, 1, 1, 1, 1, 2, 1]).unwrap();
    assert_eq!(cycle.policy, SchemeServiceCompletion::NativeCycle);
    let idle = SchemeCompletionProgress::parse(&[2, 2, -1, 0, 0, 2, 3, 0]).unwrap();
    assert!(!idle.token_owned && !idle.opportunity_pending);
}
#[test]
fn malformed_or_conflicting_custody_is_rejected() {
    for values in [
        vec![2, 1, 2, 0, 1, 1, 1, 0],
        vec![2, 1, -1, 1, 1, 1, 1, 0],
        vec![2, 1, 1, 1, 0, 1, 1, 0],
        vec![0, 1, 1, 0, 1, 1, 1, 0],
        vec![2, 1, -1, 0, 0, 1, 1, 1],
        vec![2, 1, 1, 65536, 1, 1, 1, 1],
        vec![2, 1, -1, 0, 0, 1, 1, 65537],
        vec![2, 1, -1, 0, 0, -1, 1, 0],
        vec![2, 1],
    ] {
        assert!(
            SchemeCompletionProgress::parse(&values).is_err(),
            "{values:?}"
        );
    }
}

#[test]
fn prepared_cycle_metadata() {
    let actual = SchemeCompletionProgress::parse(&[4, 2, 1, 1, 1, 1, 2, 1]).unwrap();
    assert_eq!(actual.policy, SchemeServiceCompletion::PreparedCycle);
    let pipeline = SchemeCompletionProgress::parse(&[5, 2, 1, 1, 1, 1, 2, 1]).unwrap();
    assert_eq!(pipeline.policy, SchemeServiceCompletion::PipelineCycle);
    assert!(SchemeCompletionProgress::parse(&[6, 0, -1, 0, 0, 0, 0, 0]).is_err());
}
