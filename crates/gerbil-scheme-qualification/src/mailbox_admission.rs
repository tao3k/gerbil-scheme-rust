//! Post-run qualification only; never called from the native message path.

/// Original native-mailbox guard, kept separate from process/cleanup success.
pub const HISTORICAL_P99_NS: i64 = 418_000;
/// Existing logical window, including due messages not yet materialized.
pub const ORIGINAL_WINDOW: i64 = 128;

/// Native receipts required to judge the fixed 500k/s experiment.
#[derive(Debug)]
pub struct MailboxSample {
    pub rate: u64,
    pub seconds: u64,
    pub warmup: bool,
    pub observed: bool,
    pub planned: i64,
    pub completed: i64,
    pub rejected: i64,
    pub p99_ns: i64,
    pub maximum_due_unconsumed: Option<i64>,
}

impl MailboxSample {
    /// Return every failed condition; absent coverage must never become a pass.
    #[must_use]
    pub fn failures(&self) -> Vec<&'static str> {
        let conditions = [
            (
                self.rate == 500_000 && self.seconds == 10 && !self.warmup,
                "fixed_load_not_covered",
            ),
            (!self.observed, "diagnostic_observer_enabled"),
            (
                self.planned == 5_000_000 && self.completed == self.planned && self.rejected == 0,
                "all_planned_messages_not_completed",
            ),
            (
                (1..=HISTORICAL_P99_NS).contains(&self.p99_ns),
                "historical_p99_exceeded_or_missing",
            ),
            (
                self.maximum_due_unconsumed
                    .is_some_and(|n| (0..=ORIGINAL_WINDOW).contains(&n)),
                "original_outstanding_window_exceeded_or_missing",
            ),
        ];
        conditions
            .into_iter()
            .filter_map(|(ok, reason)| (!ok).then_some(reason))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualifying() -> MailboxSample {
        MailboxSample {
            rate: 500_000,
            seconds: 10,
            warmup: false,
            observed: false,
            planned: 5_000_000,
            completed: 5_000_000,
            rejected: 0,
            p99_ns: HISTORICAL_P99_NS,
            maximum_due_unconsumed: Some(ORIGINAL_WINDOW),
        }
    }

    #[test]
    fn original_boundaries_are_inclusive() {
        assert!(qualifying().failures().is_empty());
    }

    #[test]
    fn zero_rejection_and_full_throughput_do_not_admit_recorded_candidate() {
        let sample = MailboxSample {
            p99_ns: 107_832_000,
            maximum_due_unconsumed: Some(71_549),
            ..qualifying()
        };
        assert_eq!(
            sample.failures(),
            [
                "historical_p99_exceeded_or_missing",
                "original_outstanding_window_exceeded_or_missing"
            ]
        );
    }

    #[test]
    fn favorable_diagnostic_is_not_unobserved_acceptance() {
        let sample = MailboxSample {
            p99_ns: 382_000,
            observed: true,
            maximum_due_unconsumed: Some(1027),
            ..qualifying()
        };
        assert_eq!(
            sample.failures(),
            [
                "diagnostic_observer_enabled",
                "original_outstanding_window_exceeded_or_missing"
            ]
        );
    }

    #[test]
    fn missing_receipt_or_warmup_cannot_pass() {
        let sample = MailboxSample {
            warmup: true,
            completed: 0,
            p99_ns: -1,
            maximum_due_unconsumed: None,
            ..qualifying()
        };
        assert_eq!(sample.failures().len(), 4);
    }

    #[test]
    fn successful_only_latency_does_not_hide_rejections() {
        let sample = MailboxSample {
            completed: 4_999_999,
            rejected: 1,
            ..qualifying()
        };
        assert_eq!(sample.failures(), ["all_planned_messages_not_completed"]);
    }
}
