//! Matched arrival plan; only the explicitly selected timer backend differs.
use gerbil_scheme::instances::{InstanceArrival, InstanceIssue};
use std::time::Instant;
#[derive(Debug)]
pub(super) struct Clock {
    #[cfg(feature = "arrival-clock")]
    kernel: Option<gerbil_scheme::instances::InstanceArrivalClock>,
}
impl Clock {
    pub(super) fn new(enabled: bool) -> Self {
        #[cfg(feature = "arrival-clock")]
        {
            Self {
                kernel: enabled
                    .then(|| gerbil_scheme::instances::InstanceArrivalClock::new().unwrap()),
            }
        }
        #[cfg(not(feature = "arrival-clock"))]
        {
            assert!(
                !enabled,
                "arrival-clock A/B requires the arrival-clock Cargo feature"
            );
            Self {}
        }
    }
    pub(super) async fn issue(
        &mut self,
        arrival: InstanceArrival,
        planned: Instant,
        released: Instant,
    ) -> (InstanceIssue, Option<[u128; 4]>) {
        #[cfg(feature = "arrival-clock")]
        if let Some(clock) = self.kernel.as_mut() {
            let (issue, wake) = arrival.wait_with_clock(clock).await.unwrap();
            return (
                issue,
                Some([
                    u128::from(wake.generation),
                    wake.arm_started.duration_since(released).as_nanos(),
                    wake.armed.duration_since(released).as_nanos(),
                    wake.observed.duration_since(released).as_nanos(),
                ]),
            );
        }
        let _ = released;
        tokio::time::sleep_until(tokio::time::Instant::from_std(planned)).await;
        (arrival.issue().unwrap(), None)
    }
}
