//! Indexed ready delivery with independent owner queues; no native execution authority.
use super::{InstanceError, SchemeServiceStream, types::Completed};
use std::{
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};
/// A native result identified by its original input position. Success always has
/// an actual owner; pre-entry rejection can have no owner. Order is ready-owner
/// rotation, not input order. The timestamp is native completion, not consumption.
#[derive(Debug)]
pub struct SchemeServiceReadyResult<R> {
    pub sequence: usize,
    pub owner: Option<usize>,
    pub result: Result<R, InstanceError>,
    pub native_completed: Instant,
}
#[derive(Debug)]
pub(super) struct Channels<R>(Vec<(usize, mpsc::Sender<SchemeServiceReadyResult<R>>)>);
#[derive(Debug)]
pub(super) struct Receivers<R> {
    queues: Vec<mpsc::Receiver<SchemeServiceReadyResult<R>>>,
    cursor: usize,
    remaining: usize,
}
pub(super) fn channels<R>(owners: &[usize], count: usize) -> (Arc<Channels<R>>, Receivers<R>) {
    // Any owner may claim all count jobs, but aggregate logical occupancy is
    // bounded by count original messages, not owners*count.
    let (send, queues) = owners
        .iter()
        .map(|&owner| {
            let (tx, rx) = mpsc::channel(count);
            ((owner, tx), rx)
        })
        .unzip();
    (
        Arc::new(Channels(send)),
        Receivers {
            queues,
            cursor: 0,
            remaining: count,
        },
    )
}
impl<R> Receivers<R> {
    pub(super) fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<SchemeServiceReadyResult<R>>> {
        if self.remaining == 0 {
            return Poll::Ready(None);
        }
        let mut open = false;
        for offset in 0..self.queues.len() {
            let index = (self.cursor + offset) % self.queues.len();
            match self.queues[index].poll_recv(cx) {
                Poll::Ready(Some(row)) => {
                    self.remaining -= 1;
                    self.cursor = (index + 1) % self.queues.len();
                    return Poll::Ready(Some(row));
                }
                Poll::Ready(None) => {}
                Poll::Pending => open = true,
            }
        }
        if open {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}
pub(super) enum Reply<R> {
    Single(oneshot::Sender<Completed<R>>),
    Ready {
        channels: Arc<Channels<R>>,
        sequence: usize,
    },
}
impl<R> From<oneshot::Sender<Completed<R>>> for Reply<R> {
    fn from(send: oneshot::Sender<Completed<R>>) -> Self {
        Self::Single(send)
    }
}
impl<R> Reply<R> {
    pub(super) fn send(self, value: Completed<R>, owner: Option<usize>) -> bool {
        match self {
            Self::Single(send) => send.send(value).is_ok(),
            Self::Ready { channels, sequence } => {
                let index = match owner {
                    Some(owner) => {
                        let Some(index) = channels.0.iter().position(|(id, _)| *id == owner) else {
                            return false;
                        };
                        index
                    }
                    None => sequence % channels.0.len(),
                };
                channels.0[index]
                    .1
                    .try_send(SchemeServiceReadyResult {
                        sequence,
                        owner,
                        result: value.0,
                        native_completed: value.1,
                    })
                    .is_ok()
            }
        }
    }
}
/// Opt-in owner-fair result delivery. Native credits return independently of
/// consumption. One call consumes one result; Tokio's normal cooperative budget
/// applies. This does not guarantee an OS time slice or cross-task priority.
#[derive(Debug)]
pub struct SchemeServiceReadyStream<R>(pub(super) SchemeServiceStream<R>);
impl<R> SchemeServiceReadyStream<R> {
    /// Cancel only uncommitted input. Accepted Scheme work drains on its owner.
    pub fn cancel(&self) {
        self.0.cancel();
    }
    /// Receive whichever owner has a ready result, rotating after every delivery.
    /// A timeout leaves the unread result queues intact for another call.
    /// # Errors
    /// Observation timeout; native/task errors are retained inside each indexed row.
    pub async fn next(
        &mut self,
        timeout: Duration,
    ) -> Option<Result<SchemeServiceReadyResult<R>, InstanceError>> {
        let queues = self.0.ready.as_mut()?;
        match tokio::time::timeout(timeout, std::future::poll_fn(|cx| queues.poll(cx))).await {
            Ok(None) if queues.remaining != 0 => {
                queues.remaining = 0;
                Some(Err(InstanceError::Closed))
            }
            Ok(row) => row.map(Ok),
            Err(_) => Some(Err(InstanceError::Timeout)),
        }
    }
}
#[cfg(test)]
#[path = "../../tests/unit/ready_delivery.rs"]
mod tests;
