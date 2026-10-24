//! One observer for claim progress and final native output; no execution authority.
use super::PendingGuard;
use crate::instances::{AsyncInstanceTicket, InstanceError, InstanceIssue, InstanceObservedResult};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
/// Client observation of claim, not the physical owner-claim timestamp or OS wake.
#[derive(Clone, Copy, Debug, Default)]
pub struct InstanceClaimProgress {
    pub owner: Option<usize>,
    pub observed: Option<Instant>,
    pub claim_polls: u64,
    /// Final result was observed without consuming the intermediate claim event.
    pub final_without_claim: bool,
    /// One published final result was consumed with no cooperative task budget.
    /// Diagnostic observation only; never restores a budget or native credit.
    pub ready_without_budget: bool,
}
/// Task value and timing errors remain independent, with bounded claim metadata.
#[derive(Debug)]
pub struct InstanceProgressResult<R> {
    pub observed: InstanceObservedResult<R>,
    pub claim: InstanceClaimProgress,
}
/// Explicit bounded queue receipt with a single claim/completion observation.
/// The existing pending guard is the only authority to cancel unclaimed input.
#[derive(Debug)]
pub struct ProgressSchemeTicket<R> {
    pub(super) ticket: AsyncInstanceTicket<R>,
    pub(super) placement: oneshot::Receiver<Result<usize, InstanceError>>,
    pub(super) pending: PendingGuard,
}
impl<R> ProgressSchemeTicket<R> {
    /// Wait for final native output while preserving earlier claim wakeups.
    /// # Errors
    /// Queue expiry/close, observation timeout or original native/task error.
    pub async fn wait_completed(self, timeout: Duration) -> Result<(R, Instant), InstanceError> {
        let _pending = self.pending;
        let (value, at) =
            tokio::time::timeout(timeout, Wait::<R, false>::new(self.ticket, self.placement))
                .await
                .map_err(|_| InstanceError::Timeout)?
                .map_err(|_| InstanceError::Closed)?;
        value.map(|value| (value, at))
    }
    /// Observe native completion, consumption and optional claim receipt together.
    /// Claim readiness cannot return a successful task value or release native roots.
    /// # Errors
    /// Transport timeout/close; task and timing errors are independent inner values.
    pub async fn wait_observed(
        self,
        issue: InstanceIssue,
        timeout: Duration,
    ) -> Result<InstanceProgressResult<R>, InstanceError> {
        self.observe::<false>(issue, timeout).await
    }
    /// Consume one already-published final result before cooperative deferral.
    /// Only the bounded Rust receiver read bypasses the task's cooperation
    /// budget. Pending receives, admission, timers and Scheme remain unchanged.
    /// Pair with client progress opportunities at bulk service boundaries.
    /// # Errors
    /// The same timeout/close and independent task/timing errors as `wait_observed`.
    pub async fn wait_observed_ready(
        self,
        issue: InstanceIssue,
        timeout: Duration,
    ) -> Result<InstanceProgressResult<R>, InstanceError> {
        self.observe::<true>(issue, timeout).await
    }
    async fn observe<const READY: bool>(
        self,
        issue: InstanceIssue,
        timeout: Duration,
    ) -> Result<InstanceProgressResult<R>, InstanceError> {
        let _pending = self.pending;
        let mut wait = Wait::<R, true, READY>::new(self.ticket, self.placement);
        let observed = issue.observe(&mut wait, timeout).await?;
        Ok(InstanceProgressResult {
            observed,
            claim: wait.progress,
        })
    }
}
#[derive(Debug)]
pub(super) struct Wait<R, const OBSERVE: bool = true, const READY: bool = false> {
    response: oneshot::Receiver<crate::instances::types::Completed<R>>,
    placement: Option<oneshot::Receiver<Result<usize, InstanceError>>>,
    progress: InstanceClaimProgress,
}
impl<R, const OBSERVE: bool, const READY: bool> Wait<R, OBSERVE, READY> {
    pub(super) fn new(
        ticket: AsyncInstanceTicket<R>,
        placement: oneshot::Receiver<Result<usize, InstanceError>>,
    ) -> Self {
        Self {
            response: ticket.response,
            placement: Some(placement),
            progress: InstanceClaimProgress::default(),
        }
    }
    fn final_poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<crate::instances::types::Completed<R>, oneshot::error::RecvError>> {
        let result = if READY {
            match self.response.try_recv() {
                Ok(value) => {
                    if OBSERVE {
                        self.progress.ready_without_budget =
                            !tokio::task::coop::has_budget_remaining();
                    }
                    Poll::Ready(Ok(value))
                }
                // Empty must still register/recheck through the normal receiver;
                // closure must preserve the receiver's existing error contract.
                Err(_) => Pin::new(&mut self.response).poll(cx),
            }
        } else {
            Pin::new(&mut self.response).poll(cx)
        };
        if OBSERVE && result.is_ready() {
            self.progress.final_without_claim = self.placement.is_some();
        }
        result
    }
}
impl<R, const OBSERVE: bool, const READY: bool> Future for Wait<R, OBSERVE, READY> {
    type Output = Result<crate::instances::types::Completed<R>, oneshot::error::RecvError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = this.final_poll(cx);
        if result.is_ready() {
            return result;
        }
        let Some(placement) = this.placement.as_mut() else {
            return Poll::Pending;
        };
        if OBSERVE {
            this.progress.claim_polls += 1;
        }
        match Pin::new(placement).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(claim) => {
                if OBSERVE {
                    this.progress.observed = Some(Instant::now());
                    this.progress.owner = claim.ok().and_then(Result::ok);
                }
                this.placement = None;
                // The first native poll registered this same waker. Recheck only
                // on the single claim transition, closing the claim/final race.
                // Otherwise the native receiver owns the next wake obligation.
                this.final_poll(cx)
            }
        }
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/claim_progress.rs"]
mod tests;
