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
        self.submit_native_on(owner, move |session| {
            execute_profiled(session, &items, yield_every, dispatched)
        })
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
        self.submit_native_async_on(owner, move |session| {
            execute_profiled(session, &items, yield_every, dispatched)
        })
    }
}
fn execute_profiled<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    dispatched: Instant,
) -> Result<ProfiledBatch<T>, InstanceError> {
    let collections = session.image.collection_count();
    let entered = Instant::now();
    let mut timeline = BatchTimeline {
        dispatched,
        entered,
        items_constructed: entered,
        constructed: entered,
        executed: entered,
        projected: entered,
        collections: [collections; 4],
        diagnostics: [session.diagnostics(); 5],
    };
    let values = execute(session, items, yield_every, |phase| {
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
    })?;
    Ok(ProfiledBatch { values, timeline })
}
