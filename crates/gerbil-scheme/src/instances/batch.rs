//! Bounded native numeric batches; Scheme owns handler execution and scheduling.
use super::batch_types::BatchPhase;
use super::{InstanceDispatcher, InstanceError, InstanceTicket, NativeSession};
use crate::NumericElement;

impl InstanceDispatcher {
    /// Submit one bounded turn to an application's Scheme actor-batch entry.
    /// The entry must pass the native envelope to `gerbil-rs-actor-batch!` from
    /// `:gerbil-scheme-rust/scheme/actor-batch`. Each item/result is a native
    /// numeric vector; no Scheme references leave its fixed owner.
    ///
    /// `yield_every` is the Scheme worker's cooperative item quantum (0 disables
    /// explicit yields). Handlers can also yield/wait internally. A batch cannot
    /// be preempted by another Rust queue entry. No automatic fill timer runs.
    /// # Errors
    /// Rejects empty/over-256 batches, quantum above 256, transfer bounds and
    /// closed/full queues. Scheme handler failures arrive through the ticket,
    /// after all workers have acknowledged the turn.
    pub fn submit_numeric_batch_on<T>(
        &self,
        owner: usize,
        items: Vec<Vec<T>>,
        yield_every: usize,
    ) -> Result<InstanceTicket<Vec<Vec<T>>>, InstanceError>
    where
        T: NumericElement + Send + 'static,
    {
        validate(&items, yield_every, self.maximum_input())?;
        self.submit_native_units_on(owner, items.len(), move |session| {
            execute(session, &items, yield_every, |_| {})
        })
    }
}
pub(super) fn execute<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    mut observe: impl FnMut(BatchPhase),
) -> Result<Vec<Vec<T>>, InstanceError> {
    let roots = items
        .iter()
        .map(|item| session.numeric_vector(item))
        .collect::<Result<Vec<_>, _>>()?;
    observe(BatchPhase::ItemsConstructed);
    let references: Vec<_> = roots.iter().collect();
    let input = session.vector(&references)?;
    let quantum = session.numeric_vector(&[i64::try_from(yield_every)
        .map_err(|_| InstanceError::InvalidConfig("native batch quantum"))?])?;
    let envelope = session.vector(&[&quantum, &input])?;
    observe(BatchPhase::Constructed);
    let output = session.call(&envelope)?;
    observe(BatchPhase::Executed);
    let values = project(session, &output, items.len())?;
    observe(BatchPhase::Projected);
    Ok(values)
}
/// Opt-in owner producers use the native bulk constructor when their image
/// supports it. Older ABI-v3 images keep the existing rooted native path.
pub(super) fn execute_native<T: NumericElement>(
    session: &NativeSession<'_>,
    items: &[Vec<T>],
    yield_every: usize,
    mut observe: impl FnMut(BatchPhase),
) -> Result<Vec<Vec<T>>, InstanceError> {
    if session.image.native.numeric_batch_create.is_none() {
        return execute(session, items, yield_every, observe);
    }
    let envelope = session.numeric_batch_envelope(items, yield_every)?;
    observe(BatchPhase::ConstructedTogether);
    let output = session.call(&envelope)?;
    observe(BatchPhase::Executed);
    let values = project(session, &output, items.len())?;
    observe(BatchPhase::Projected);
    Ok(values)
}
pub(super) fn project<T: NumericElement>(
    session: &NativeSession<'_>,
    output: &super::NativeRoot<'_>,
    count: usize,
) -> Result<Vec<Vec<T>>, InstanceError> {
    #[cfg(feature = "tokio-dispatch")]
    if session.entry_id().is_some() {
        return session
            .numeric_batch_snapshot::<T>(output, count)?
            .copy(session);
    }
    let mut total = 0_usize;
    (0..count)
        .map(|index| {
            let values = session.copy_numeric_element_bounded::<T>(
                output,
                index,
                session.maximum_output - total,
            )?;
            total = total
                .checked_add(std::mem::size_of_val(values.as_slice()))
                .ok_or(InstanceError::InvalidConfig("native batch output overflow"))?;
            if total > session.maximum_output {
                return Err(InstanceError::InvalidConfig("native batch output bound"));
            }
            Ok(values)
        })
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn validate<T>(
    items: &[Vec<T>],
    quantum: usize,
    maximum: usize,
) -> Result<(), InstanceError> {
    if items.is_empty() || items.len() > 256 || quantum > 256 {
        return Err(InstanceError::InvalidConfig(
            "native batch item/quantum bound",
        ));
    }
    let bytes = input_size(items);
    if bytes.is_none_or(|bytes| bytes > maximum) {
        return Err(InstanceError::InvalidConfig("native batch input bound"));
    }
    Ok(())
}

pub(super) fn input_size<T>(items: &[Vec<T>]) -> Option<usize> {
    items
        .iter()
        .try_fold((items.len() + 3).checked_mul(8)?, |sum, item| {
            sum.checked_add(std::mem::size_of_val(item.as_slice()))
        })
}
