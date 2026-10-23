//! Optional Rust scheduling over native owners; Scheme values never enter workers.
use gerbil_scheme::instances::{AsyncInstanceTicket, InstanceError};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
type Indexed<T> = (usize, T);
type Group<T> = (usize, Vec<Indexed<T>>);
/// One owned native response and its owner completion timestamp.
pub type OwnerResult<R> = Result<(R, Instant), InstanceError>;
fn groups<T>(jobs: Vec<(usize, T)>) -> Vec<Group<T>> {
    let mut groups: BTreeMap<usize, Vec<Indexed<T>>> = BTreeMap::new();
    for (index, (owner, item)) in jobs.into_iter().enumerate() {
        groups.entry(owner).or_default().push((index, item));
    }
    groups.into_iter().collect()
}
fn restore<T>(rows: Vec<Indexed<T>>, count: usize) -> Vec<T> {
    let mut slots: Vec<_> = (0..count).map(|_| None).collect();
    for (index, result) in rows {
        slots[index] = Some(result);
    }
    slots
        .into_iter()
        .map(|item| item.expect("one result per original request"))
        .collect()
}
/// Submit owner groups in parallel, preserving each owner's order and result order.
/// The callback performs admission only; waiting stays with the caller. Native
/// queue bounds, close fencing and errors remain owned by the callback's API.
#[must_use]
pub fn rayon_owner_submit<T, R, F>(
    pool: &rayon::ThreadPool,
    jobs: Vec<(usize, T)>,
    submit: F,
) -> Vec<Result<R, InstanceError>>
where
    T: Send,
    R: Send,
    F: Fn(usize, T) -> Result<R, InstanceError> + Sync,
{
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    let count = jobs.len();
    let rows = pool.install(|| {
        groups(jobs)
            .into_par_iter()
            .flat_map_iter(|(owner, group)| {
                let submit = &submit;
                group.into_iter().map(move |(index, item)| {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        submit(owner, item)
                    }))
                    .unwrap_or(Err(InstanceError::TaskPanicked));
                    (index, result)
                })
            })
            .collect()
    });
    restore(rows, count)
}
/// Await one Rust task per owner, with all requests admitted before its waits.
/// Request results retain input order. Each request keeps the same native API,
/// deadline and errors; a timeout never cancels accepted Scheme execution.
/// Submission panics affect one request and do not abandon its accepted peers.
pub async fn tokio_owner_window<T, R, F>(
    jobs: Vec<(usize, T)>,
    released: Instant,
    timeout: Duration,
    submit: F,
) -> Vec<OwnerResult<R>>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError> + Send + Sync + 'static,
{
    tokio_owner_window_observed(jobs, WindowBudget { released, timeout }, submit, |_, _| {}).await
}
/// Rust response boundary; neither stage executes or preempts Scheme.
#[derive(Clone, Copy, Debug)]
pub enum CollectionStage {
    /// A native ticket was observed by its owner-group task.
    Received,
    /// The completed group was collected by the caller.
    Collected,
}
/// Diagnostic counterpart with caller-provided clock hooks at result boundaries.
/// Hooks are synchronous; the normal API uses an empty hook with no clock reads.
/// Poll inside a Tokio runtime with its time driver enabled and drain the future.
pub async fn tokio_owner_window_observed<T, R, F, O>(
    jobs: Vec<(usize, T)>,
    budget: WindowBudget,
    submit: F,
    observe: O,
) -> Vec<OwnerResult<R>>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError> + Send + Sync + 'static,
    O: Fn(usize, CollectionStage) + Send + Sync + 'static,
{
    let count = jobs.len();
    let submit = Arc::new(submit);
    let observe = Arc::new(observe);
    let mut tasks = tokio::task::JoinSet::new();
    for group in groups(jobs) {
        tasks.spawn(drain_owner_group(
            group,
            budget,
            Arc::clone(&submit),
            Arc::clone(&observe),
        ));
    }
    collect_groups(tasks, count, observe).await
}
/// One immutable timing origin and finite wait budget for an offered window.
#[derive(Clone, Copy, Debug)]
pub struct WindowBudget {
    /// Common release timestamp, shared by all owner submissions and waits.
    pub released: Instant,
    /// Original window timeout; accepted native work is never cancelled.
    pub timeout: Duration,
}
impl WindowBudget {
    pub(super) fn remaining(self) -> Duration {
        self.timeout.saturating_sub(self.released.elapsed())
    }
    pub(super) fn expired(self) -> bool {
        self.released.elapsed() >= self.timeout
    }
}
type Pending<R> = Indexed<Result<AsyncInstanceTicket<R>, InstanceError>>;
fn admit_owner_group<T, R, F>(
    (owner, group): Group<T>,
    budget: WindowBudget,
    submit: &F,
) -> Vec<Pending<R>>
where
    F: Fn(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError>,
{
    group
        .into_iter()
        .map(|(index, item)| {
            if budget.expired() {
                return (index, Err(InstanceError::Timeout));
            }
            let ticket =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| submit(owner, item)))
                    .unwrap_or(Err(InstanceError::TaskPanicked));
            (index, ticket)
        })
        .collect()
}
async fn drain_owner_group<T, R, F, O>(
    group: Group<T>,
    budget: WindowBudget,
    submit: Arc<F>,
    observe: Arc<O>,
) -> Vec<Indexed<OwnerResult<R>>>
where
    F: Fn(usize, T) -> Result<AsyncInstanceTicket<R>, InstanceError>,
    O: Fn(usize, CollectionStage),
{
    let tickets = admit_owner_group(group, budget, submit.as_ref());
    let mut results = Vec::new();
    for (index, ticket) in tickets {
        let result = match ticket {
            Ok(ticket) => ticket.wait_completed(budget.remaining()).await,
            Err(error) => Err(error),
        };
        observe(index, CollectionStage::Received);
        results.push((index, result));
    }
    results
}
async fn collect_groups<R, O>(
    mut tasks: tokio::task::JoinSet<Vec<Indexed<OwnerResult<R>>>>,
    count: usize,
    observe: Arc<O>,
) -> Vec<OwnerResult<R>>
where
    R: 'static,
    O: Fn(usize, CollectionStage),
{
    let mut slots: Vec<_> = (0..count).map(|_| None).collect();
    while let Some(result) = tasks.join_next().await {
        let Ok(rows) = result else {
            continue;
        };
        for (index, value) in rows {
            observe(index, CollectionStage::Collected);
            slots[index] = Some(value);
        }
    }
    slots
        .into_iter()
        .map(|item| item.unwrap_or(Err(InstanceError::TaskPanicked)))
        .collect()
}
