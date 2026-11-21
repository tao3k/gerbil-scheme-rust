//! Opt-in measurement uses exactly the normal native batch execution path.
use super::{
    BatchTimeline, InstanceDispatcher, InstanceError, InstanceTicket, NativeSession, ProfiledBatch,
    batch::{execute, validate},
    batch_types::BatchPhase,
};
use crate::NumericElement;
use std::time::Instant;
impl InstanceDispatcher {
    /// Submit a native batch with construction, Scheme-entry and projection clocks.
    /// The regular API has no phase-clock reads. Pair with `wait_completed` to
    /// measure final root retirement separately. See `BatchTimeline` for boundaries.
    /// # Errors
    /// Same admission, transfer and Scheme-handler errors as `submit_numeric_batch_on`.
    pub fn submit_numeric_batch_profiled_on<T>(
        &self,
        owner: usize,
        items: Vec<Vec<T>>,
        yield_every: usize,
    ) -> Result<InstanceTicket<ProfiledBatch<T>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        validate(&items, yield_every, self.maximum_input())?;
        let dispatched = Instant::now();
        self.submit_native_units_on(owner, items.len(), move |session| {
            execute_profiled(session, &items, yield_every, dispatched)
        })
    }

    /// Measure the existing Scheme turn after finite weighted admission.
    /// Markers observe service duration; they do not enforce wall-clock preemption.
    /// # Errors
    /// Same native batch validation, work-budget and finite admission failures.
    #[cfg(feature = "tokio-dispatch")]
    pub async fn submit_numeric_batch_profiled_backpressured<T>(
        &self,
        items: Vec<Vec<T>>,
        admission: super::BatchAdmission,
    ) -> Result<super::AsyncInstanceTicket<ProfiledBatch<T>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        validate(&items, admission.yield_every, self.maximum_input())?;
        let dispatched = Instant::now();
        self.submit_native_weighted_backpressured_on(
            admission.owner,
            items.len(),
            move |session| execute_profiled(session, &items, admission.yield_every, dispatched),
            admission.timeout,
        )
        .await
    }

    /// Submit the same profiled native batch with asynchronous result delivery.
    /// # Errors
    /// Same bounds, admission and native errors as the synchronous profiled API.
    #[cfg(feature = "tokio-dispatch")]
    pub fn submit_numeric_batch_profiled_async_on<T>(
        &self,
        owner: usize,
        items: Vec<Vec<T>>,
        yield_every: usize,
    ) -> Result<super::AsyncInstanceTicket<ProfiledBatch<T>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        validate(&items, yield_every, self.maximum_input())?;
        let dispatched = Instant::now();
        self.submit_native_async_units_on(owner, items.len(), move |session| {
            execute_profiled(session, &items, yield_every, dispatched)
        })
    }
}
pub(super) fn execute_profiled<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    dispatched: Instant,
) -> Result<ProfiledBatch<T>, InstanceError> {
    execute_profiled_impl(session, items, yield_every, dispatched, false)
}
#[cfg(feature = "tokio-dispatch")]
pub(super) fn execute_profiled_native<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    dispatched: Instant,
) -> Result<ProfiledBatch<T>, InstanceError> {
    execute_profiled_impl(session, items, yield_every, dispatched, true)
}
fn execute_profiled_impl<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    dispatched: Instant,
    native: bool,
) -> Result<ProfiledBatch<T>, InstanceError> {
    let native = native && session.image.native.numeric_batch_create.is_some();
    let collections = session.image.collection_count();
    let entered = Instant::now();
    let mut timeline = BatchTimeline {
        dispatched,
        entered,
        native_constructor: native,
        items_constructed: entered,
        constructed: entered,
        executed: entered,
        projected: entered,
        retired: entered,
        collections_after_retirement: collections,
        diagnostics_after_retirement: None,
        collections: [collections; 4],
        diagnostics: [session.diagnostics(); 5],
    };
    let mut observe = |phase| {
        let now = Instant::now();
        match phase {
            BatchPhase::ItemsConstructed => {
                timeline.items_constructed = now;
                timeline.diagnostics[1] = session.diagnostics();
            }
            BatchPhase::Constructed => {
                timeline.constructed = now;
                timeline.collections[1] = session.image.collection_count();
                timeline.diagnostics[2] = session.diagnostics();
            }
            BatchPhase::ConstructedTogether => {
                timeline.items_constructed = now;
                timeline.constructed = now;
                timeline.collections[1] = session.gc_collections();
                let diagnostics = session.diagnostics();
                timeline.diagnostics[1] = diagnostics;
                timeline.diagnostics[2] = diagnostics;
            }
            BatchPhase::Executed => {
                timeline.executed = now;
                timeline.collections[2] = session.image.collection_count();
                timeline.diagnostics[3] = session.diagnostics();
            }
            BatchPhase::Projected => {
                timeline.projected = now;
                timeline.collections[3] = session.image.collection_count();
                timeline.diagnostics[4] = session.diagnostics();
            }
        }
    };
    let values = if native {
        super::batch::execute_native(session, items, yield_every, &mut observe)?
    } else {
        execute(session, items, yield_every, &mut observe)?
    };
    timeline.retired = Instant::now();
    timeline.collections_after_retirement = session.gc_collections();
    timeline.diagnostics_after_retirement = session.diagnostics();
    Ok(ProfiledBatch { values, timeline })
}
