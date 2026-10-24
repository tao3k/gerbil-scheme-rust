//! Structured client progress before a bulk service turn, never Scheme scheduling.
use std::{
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};
#[derive(Debug)]
struct Signal {
    ready: AtomicBool,
    parent: Mutex<Option<Waker>>,
}
impl Signal {
    fn arm(&self, waker: &Waker) {
        let mut parent = self
            .parent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if parent.as_ref().is_none_or(|old| !old.will_wake(waker)) {
            *parent = Some(waker.clone());
        }
    }
    fn retire(&self) {
        self.parent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
    fn notify(&self) {
        if !self.ready.swap(true, Ordering::AcqRel) {
            let parent = self
                .parent
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            // Wake outside the registration lock, including arbitrary caller wakers.
            if let Some(parent) = parent {
                parent.wake();
            }
        }
    }
}
impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.notify();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.notify();
    }
}
/// Own one client progress future alongside bounded bulk service turns. Actual
/// progress readiness is checked before each work poll, including immediately
/// ready work. Unsignalled progress is not repeatedly polled. This creates no
/// task/thread, owns no native credit and cannot guarantee an OS resumption.
/// Call `wait` around registration and every result receive; doing arbitrary
/// unbounded work between calls is outside this opportunity contract.
pub struct InstanceClientProgressDriver<P: Future> {
    progress: Pin<Box<P>>,
    output: Option<P::Output>,
    done: bool,
    signal: Arc<Signal>,
}
impl<P: Future> fmt::Debug for InstanceClientProgressDriver<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstanceClientProgressDriver")
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}
impl<P: Future> InstanceClientProgressDriver<P> {
    #[must_use]
    pub fn new(progress: P) -> Self {
        Self {
            progress: Box::pin(progress),
            output: None,
            done: false,
            signal: Arc::new(Signal {
                ready: AtomicBool::new(true),
                parent: Mutex::new(None),
            }),
        }
    }
    fn poll_progress(&mut self, cx: &mut Context<'_>) {
        if self.done {
            return;
        }
        self.signal.arm(cx.waker());
        if !self.signal.ready.swap(false, Ordering::AcqRel) {
            return;
        }
        let waker = Waker::from(Arc::clone(&self.signal));
        if let Poll::Ready(value) = self
            .progress
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
        {
            self.output = Some(value);
            self.done = true;
            self.signal.retire();
        }
    }
    /// Offer signalled client progress before every work poll. Neither future's
    /// ready result acknowledges another's completion or frees native ownership.
    pub async fn wait<W: Future>(&mut self, work: W) -> W::Output {
        let mut work = std::pin::pin!(work);
        std::future::poll_fn(|cx| {
            self.poll_progress(cx);
            work.as_mut().poll(cx)
        })
        .await
    }
    /// Finish outstanding client progress after the last bulk turn. A cached
    /// result returns once without repolling the completed future.
    pub async fn finish(mut self) -> P::Output {
        std::future::poll_fn(|cx| {
            self.poll_progress(cx);
            self.output.take().map_or(Poll::Pending, Poll::Ready)
        })
        .await
    }
}
impl<P: Future> Drop for InstanceClientProgressDriver<P> {
    fn drop(&mut self) {
        self.signal.retire();
    }
}
#[cfg(test)]
#[path = "../../tests/unit/client_driver.rs"]
mod tests;
