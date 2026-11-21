//! Completion-driven native admission without per-request executor tasks.
use crate::owner_dispatch::{OwnerResult, WindowBudget};
use gerbil_scheme::instances::{AsyncInstanceTicket, InstanceError};
use std::{
    collections::{BTreeMap, VecDeque},
    future::{Future, poll_fn},
    pin::Pin,
    task::{Context, Poll},
};
/// Explicit per-invocation limits; this does not reserve capacity from other callers.
#[derive(Clone, Copy, Debug)]
pub struct FlowControl {
    /// Existing common release timestamp and finite observation deadline.
    pub budget: WindowBudget,
    /// Maximum accepted tickets retained by this driver at once; must be positive.
    pub maximum_in_flight: usize,
}
/// Optional observation boundaries on the calling future, never inside Scheme.
#[derive(Clone, Copy, Debug)]
pub enum FlowStage {
    /// About to invoke the existing native admission API.
    Submitting,
    /// The admission callback has returned, successfully or with rejection.
    Admitted,
    /// A result has been observed and placed in its original output slot.
    Collected,
}
/// Drive routed native requests with bounded credits and no spawned Rust tasks.
/// Each owner's input order and original output positions are preserved. Poll in
/// a Tokio runtime with time enabled and drain the future. Timeout or dropping
/// the future does not cancel accepted native execution. The callback performs
/// admission only; native types, execution and retirement remain on the owner.
/// This is opt-in and does not replace existing dispatcher selections.
pub async fn owner_flow_window<T, R, F>(
    jobs: Vec<(usize, T)>,
    control: FlowControl,
    submit: F,
) -> Vec<OwnerResult<R>>
where
    R: Send + 'static,
    F: FnMut(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError>,
{
    owner_flow_window_observed(jobs, control, submit, |_, _| {}).await
}
/// Diagnostic counterpart. Hooks must not panic or block; the normal API's
/// empty hook reads no clocks. Ready completions refill this invocation's credit;
/// queued caller inputs do not occupy additional native queue slots.
pub async fn owner_flow_window_observed<T, R, F, O>(
    jobs: Vec<(usize, T)>,
    control: FlowControl,
    mut submit: F,
    mut observe: O,
) -> Vec<OwnerResult<R>>
where
    R: Send + 'static,
    F: FnMut(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError>,
    O: FnMut(usize, FlowStage),
{
    drive(
        jobs,
        control,
        |owner, item| std::future::ready(submit(owner, item)),
        &mut observe,
        false,
        false,
    )
    .await
}
/// Completion flow whose callback may await pool-wide owner capacity. There is
/// at most one pending admission per owner per invocation, so a producer cannot
/// prequeue its entire backlog ahead of other producers' reservations. Poll in
/// a Tokio runtime with time enabled. Accepted native work is never cancelled.
/// Callback futures must perform admission only and return the existing ticket.
pub async fn owner_flow_backpressured<T, R, F, A>(
    jobs: Vec<(usize, T)>,
    control: FlowControl,
    submit: F,
) -> Vec<OwnerResult<R>>
where
    R: Send + 'static,
    F: FnMut(usize, T) -> A,
    A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
{
    drive(jobs, control, submit, &mut |_, _| {}, true, false).await
}
/// Completion-ready variant for concurrent Scheme services. A slow earlier
/// request does not block observing a later result or refilling client credits.
/// Final output positions and per-owner admission order remain unchanged.
/// This explicit API changes neither native capacity nor Scheme scheduling.
pub async fn owner_flow_ready_backpressured<T, R, F, A>(
    jobs: Vec<(usize, T)>,
    control: FlowControl,
    submit: F,
) -> Vec<OwnerResult<R>>
where
    R: Send + 'static,
    F: FnMut(usize, T) -> A,
    A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
{
    drive(jobs, control, submit, &mut |_, _| {}, true, true).await
}
async fn drive<T, R, F, A, O>(
    jobs: Vec<(usize, T)>,
    control: FlowControl,
    mut submit: F,
    observe: &mut O,
    bounded_admission: bool,
    ready_completion: bool,
) -> Vec<OwnerResult<R>>
where
    R: Send + 'static,
    F: FnMut(usize, T) -> A,
    A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
    O: FnMut(usize, FlowStage),
{
    let mut flow = Flow::new(jobs, control, bounded_admission, ready_completion);
    poll_fn(|context| flow.poll(context, &mut submit, observe)).await;
    flow.finish()
}
type Admission<R> =
    Pin<Box<dyn Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send>>;
struct Admitting<R> {
    index: usize,
    future: Admission<R>,
}
type Response<R> = Pin<Box<dyn Future<Output = OwnerResult<R>> + Send>>;
struct Pending<R> {
    index: usize,
    response: Response<R>,
}
struct Lane<T, R> {
    owner: usize,
    waiting: VecDeque<(usize, T)>,
    active: VecDeque<Pending<R>>,
    admitting: Option<Admitting<R>>,
}
struct Flow<T, R> {
    lanes: Vec<Lane<T, R>>,
    slots: Vec<Option<OwnerResult<R>>>,
    control: FlowControl,
    remaining: usize,
    active: usize,
    cursor: usize,
    quota: usize,
    bounded_admission: bool,
    ready_completion: bool,
}
impl<T, R: Send + 'static> Flow<T, R> {
    fn new(
        jobs: Vec<(usize, T)>,
        control: FlowControl,
        bounded_admission: bool,
        ready_completion: bool,
    ) -> Self {
        let count = jobs.len();
        let mut waiting: BTreeMap<usize, VecDeque<(usize, T)>> = BTreeMap::new();
        for (index, (owner, item)) in jobs.into_iter().enumerate() {
            waiting.entry(owner).or_default().push_back((index, item));
        }
        let quota = control.maximum_in_flight.div_ceil(waiting.len().max(1));
        Self {
            lanes: waiting
                .into_iter()
                .map(|(owner, waiting)| Lane {
                    owner,
                    waiting,
                    active: VecDeque::new(),
                    admitting: None,
                })
                .collect(),
            slots: (0..count).map(|_| None).collect(),
            control,
            remaining: count,
            active: 0,
            cursor: 0,
            quota,
            bounded_admission,
            ready_completion,
        }
    }
    fn finish(self) -> Vec<OwnerResult<R>> {
        self.slots
            .into_iter()
            .map(|slot| slot.expect("one result per offered request"))
            .collect()
    }
    fn begin_admission<F, A, O>(&mut self, lane: usize, submit: &mut F, observe: &mut O)
    where
        F: FnMut(usize, T) -> A,
        A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
        O: FnMut(usize, FlowStage),
    {
        let owner = &mut self.lanes[lane];
        let (index, item) = owner.waiting.pop_front().expect("waiting input");
        observe(index, FlowStage::Submitting);
        let future: Admission<R> = if self.control.budget.expired() {
            Box::pin(std::future::ready(Err(InstanceError::Timeout)))
        } else {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                submit(owner.owner, item)
            })) {
                Ok(future) if self.bounded_admission => {
                    let budget = self.control.budget;
                    Box::pin(async move {
                        if budget.expired() {
                            return Err(InstanceError::Timeout);
                        }
                        tokio::time::timeout(budget.remaining(), future)
                            .await
                            .unwrap_or(Err(InstanceError::Timeout))
                    })
                }
                Ok(future) => Box::pin(future),
                Err(_) => Box::pin(std::future::ready(Err(InstanceError::TaskPanicked))),
            }
        };
        owner.admitting = Some(Admitting { index, future });
        self.active += 1;
    }
    fn poll_admission<O>(&mut self, lane: usize, context: &mut Context<'_>, observe: &mut O) -> bool
    where
        O: FnMut(usize, FlowStage),
    {
        let owner = &mut self.lanes[lane];
        let Some(admitting) = owner.admitting.as_mut() else {
            return false;
        };
        let poll = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            admitting.future.as_mut().poll(context)
        }))
        .unwrap_or(Poll::Ready(Err(InstanceError::TaskPanicked)));
        let Poll::Ready(ticket) = poll else {
            return false;
        };
        let index = admitting.index;
        owner.admitting = None;
        observe(index, FlowStage::Admitted);
        let budget = self.control.budget;
        owner.active.push_back(Pending {
            index,
            response: Box::pin(async move { ticket?.wait_completed(budget.remaining()).await }),
        });
        true
    }
    fn fill<F, A, O>(&mut self, context: &mut Context<'_>, submit: &mut F, observe: &mut O)
    where
        F: FnMut(usize, T) -> A,
        A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
        O: FnMut(usize, FlowStage),
    {
        // Poll reservations even if they occupy the last invocation credits.
        for lane in 0..self.lanes.len() {
            self.poll_admission(lane, context, observe);
        }
        while self.active < self.control.maximum_in_flight {
            let next = (0..self.lanes.len())
                .map(|offset| (self.cursor + offset) % self.lanes.len())
                .find(|&lane| self.can_admit(lane) && self.lanes[lane].active.len() < self.quota)
                .or_else(|| {
                    (0..self.lanes.len())
                        .map(|offset| (self.cursor + offset) % self.lanes.len())
                        .find(|&lane| self.can_admit(lane))
                });
            let Some(lane) = next else {
                break;
            };
            self.cursor = (lane + 1) % self.lanes.len();
            self.begin_admission(lane, submit, observe);
            self.poll_admission(lane, context, observe);
        }
    }
    fn can_admit(&self, lane: usize) -> bool {
        !self.lanes[lane].waiting.is_empty() && self.lanes[lane].admitting.is_none()
    }
    fn collect_ready<O>(&mut self, context: &mut Context<'_>, observe: &mut O) -> bool
    where
        O: FnMut(usize, FlowStage),
    {
        let mut progress = false;
        for lane in &mut self.lanes {
            // Register every pending receiver when opted in; otherwise an
            // unready head can hide completed peers and strand client credits.
            let width = if self.ready_completion {
                lane.active.len()
            } else {
                1
            };
            let ready =
                lane.active
                    .iter_mut()
                    .take(width)
                    .enumerate()
                    .find_map(|(position, pending)| {
                        match pending.response.as_mut().poll(context) {
                            Poll::Ready(result) => Some((position, result)),
                            Poll::Pending => None,
                        }
                    });
            let Some((position, result)) = ready else {
                continue;
            };
            let index = lane.active.remove(position).unwrap().index;
            self.slots[index] = Some(result);
            observe(index, FlowStage::Collected);
            self.remaining -= 1;
            self.active -= 1;
            progress = true;
        }
        progress
    }
    fn poll<F, A, O>(
        &mut self,
        context: &mut Context<'_>,
        submit: &mut F,
        observe: &mut O,
    ) -> Poll<()>
    where
        F: FnMut(usize, T) -> A,
        A: Future<Output = Result<AsyncInstanceTicket<R>, InstanceError>> + Send + 'static,
        O: FnMut(usize, FlowStage),
    {
        if self.control.maximum_in_flight == 0 {
            for slot in &mut self.slots {
                *slot = Some(Err(InstanceError::InvalidConfig(
                    "flow credit must be positive",
                )));
            }
            return Poll::Ready(());
        }
        // Bound synchronous completion work per poll; no Scheme scheduling changes.
        for _ in 0..self.control.maximum_in_flight {
            self.fill(context, submit, observe);
            if self.remaining == 0 {
                return Poll::Ready(());
            }
            if !self.collect_ready(context, observe) {
                return Poll::Pending;
            }
        }
        context.waker().wake_by_ref();
        Poll::Pending
    }
}
