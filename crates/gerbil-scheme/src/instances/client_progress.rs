//! Client arrival and consumption boundaries; no Scheme roots or native credits.
use super::{AsyncInstanceTicket, InstanceError};
use std::time::{Duration, Instant};

/// A planned arrival before its client timer wait. This is not native admission.
#[derive(Debug)]
pub struct InstanceArrival {
    planned: Instant,
    wait_started: Instant,
    previous_observed: Option<Instant>,
}
/// Start of one actual submission attempt, before input preparation/admission.
/// Dropping this token neither cancels accepted work nor owns any native credit.
#[derive(Debug)]
pub struct InstanceIssue {
    arrival: InstanceArrival,
    issued: Instant,
    timer_armed: Option<bool>,
}
/// One monotonic client/native timeline. Consumption means receiving the Rust
/// result, before application processing. Native completion is not consumption.
#[derive(Clone, Copy, Debug)]
pub struct InstanceClientProgress {
    pub planned: Instant,
    pub wait_started: Instant,
    pub issued: Instant,
    pub native_completed: Instant,
    pub client_observed: Instant,
    pub response_poll_started: Instant,
    pub response_poll_ready: Instant,
    pub pending_polls: u64,
    pub previous_observed: Option<Instant>,
    /// None: caller supplied the wait. Some: an explicit arrival wait API.
    pub timer_armed: Option<bool>,
}
impl InstanceArrival {
    /// Record the boundary before waiting. All instants use this process clock.
    /// # Errors
    /// Rejects a predecessor observation later than this boundary.
    pub fn new(
        planned: Instant,
        previous_observed: Option<Instant>,
    ) -> Result<Self, InstanceError> {
        let wait_started = Instant::now();
        if previous_observed.is_some_and(|t| t > wait_started) {
            return Err(InstanceError::InvalidConfig(
                "future predecessor observation",
            ));
        }
        Ok(Self {
            planned,
            wait_started,
            previous_observed,
        })
    }
    /// Mark submission after a caller-owned wait, retaining that original path.
    /// # Errors
    /// A request cannot be issued before its planned arrival.
    pub fn issue(self) -> Result<InstanceIssue, InstanceError> {
        self.issue_at(Instant::now(), None)
    }
    /// Wait only when the deadline is still future at the actual poll. Already
    /// due work goes directly to admission, without allocating another timer.
    /// This does not bypass admission, Tokio result waits or Scheme scheduling.
    /// # Errors
    /// Reports a nonmonotonic/early timer completion.
    pub async fn wait_until_due(self) -> Result<InstanceIssue, InstanceError> {
        let now = Instant::now();
        if now >= self.planned {
            return self.issue_at(now, Some(false));
        }
        tokio::time::sleep_until(tokio::time::Instant::from_std(self.planned)).await;
        self.issue_at(Instant::now(), Some(true))
    }
    /// Always arm the explicit kernel clock, even for an already-due plan.
    /// Clock observation is not submission or admission. Issue is recorded after
    /// timer teardown; existing queue/owner capacity checks still apply.
    /// # Errors
    /// Unsupported clock/runtime I/O, timer failure or invalid temporal order.
    #[cfg(feature = "arrival-clock")]
    pub async fn wait_with_clock(
        self,
        clock: &mut super::InstanceArrivalClock,
    ) -> Result<(InstanceIssue, super::InstanceArrivalWake), InstanceError> {
        let wake = clock
            .wait_until(self.planned)
            .await
            .map_err(|error| InstanceError::ArrivalClock(error.to_string()))?;
        let issue = self.issue_at(Instant::now(), Some(true))?;
        Ok((issue, wake))
    }
    fn issue_at(
        self,
        issued: Instant,
        timer_armed: Option<bool>,
    ) -> Result<InstanceIssue, InstanceError> {
        if issued < self.planned || issued < self.wait_started {
            return Err(InstanceError::InvalidConfig(
                "arrival issued before eligibility",
            ));
        }
        Ok(InstanceIssue {
            arrival: self,
            issued,
            timer_armed,
        })
    }
}
impl InstanceIssue {
    #[must_use]
    pub fn issued_at(&self) -> Instant {
        self.issued
    }
}
/// Native task failure remains observable with its completion timeline. Only
/// transport/observation errors use the outer wait result.
#[derive(Debug)]
pub struct InstanceObservedResult<R> {
    pub result: Result<R, InstanceError>,
    /// Invalid observation metadata never discards a successfully returned value.
    pub progress: Result<InstanceClientProgress, InstanceError>,
}
impl<R> AsyncInstanceTicket<R> {
    /// Receive a result with planned/issued/native-ready/client-observed times.
    /// Issue is the start of the client's attempt, not the owner's acceptance.
    /// Timing out preserves the existing accepted-work drain contract.
    /// # Errors
    /// Transport timeout or closed receiver. Native failure and invalid timing
    /// are independent inner results, preserving any successfully owned value.
    pub async fn wait_observed(
        self,
        issue: InstanceIssue,
        timeout: Duration,
    ) -> Result<InstanceObservedResult<R>, InstanceError> {
        issue.observe(self.response, timeout).await
    }
}
impl InstanceIssue {
    pub(super) async fn observe<R, F>(
        self,
        response: F,
        timeout: Duration,
    ) -> Result<InstanceObservedResult<R>, InstanceError>
    where
        F: std::future::Future<
                Output = Result<super::types::Completed<R>, tokio::sync::oneshot::error::RecvError>,
            > + Unpin,
    {
        let issue = self;
        let mut response = response;
        let mut poll_started = None;
        let mut poll_ready = None;
        let mut pending_polls = 0;
        let receive = std::future::poll_fn(|cx| {
            let start = Instant::now();
            let result = std::pin::Pin::new(&mut response).poll(cx);
            if result.is_ready() {
                poll_started = Some(start);
                poll_ready = Some(Instant::now());
            } else {
                pending_polls += 1;
            }
            result
        });
        let (result, native_completed) = tokio::time::timeout(timeout, receive)
            .await
            .map_err(|_| InstanceError::Timeout)?
            .map_err(|_| InstanceError::Closed)?;
        let client_observed = Instant::now();
        let progress = (|| {
            let (Some(response_poll_started), Some(response_poll_ready)) =
                (poll_started, poll_ready)
            else {
                return Err(InstanceError::InvalidConfig("missing ready receiver poll"));
            };
            if native_completed < issue.issued
                || native_completed > response_poll_ready
                || response_poll_started > response_poll_ready
                || response_poll_ready > client_observed
            {
                return Err(InstanceError::InvalidConfig("client/native timeline order"));
            }
            Ok(InstanceClientProgress {
                planned: issue.arrival.planned,
                wait_started: issue.arrival.wait_started,
                issued: issue.issued,
                native_completed,
                client_observed,
                response_poll_started,
                response_poll_ready,
                pending_polls,
                previous_observed: issue.arrival.previous_observed,
                timer_armed: issue.timer_armed,
            })
        })();
        Ok(InstanceObservedResult { result, progress })
    }
}

impl InstanceClientProgress {
    /// Predecessor consumption already late against this request's plan.
    #[must_use]
    pub fn serial_backlog(&self) -> Duration {
        self.previous_observed.map_or(Duration::ZERO, |t| {
            t.saturating_duration_since(self.planned)
        })
    }
    /// Client processing between predecessor consumption and the timer boundary.
    #[must_use]
    pub fn before_wait(&self) -> Duration {
        self.wait_started.saturating_duration_since(
            self.previous_observed
                .unwrap_or(self.planned)
                .max(self.planned),
        )
    }
    /// Deadline eligibility to actual resumption/issue, excluding the two above.
    /// Includes OS scheduling; this does not assert Tokio-internal runnable time.
    #[must_use]
    pub fn deadline_resume(&self) -> Duration {
        self.issued
            .duration_since(self.planned.max(self.wait_started))
    }
    #[must_use]
    pub fn delivery(&self) -> Duration {
        self.client_observed.duration_since(self.native_completed)
    }
}
