//! Completion-driven bounded admission; every Scheme operation stays on its fixed owner.
use super::super::Input;
use super::{
    Answer, Command, GATE, InstanceError, InstanceService, NativeOutput, Output, Registry,
    trace::prepare_named,
};
use gerbil_scheme::instances::InstanceDispatcher;
use std::{ffi::CStr, sync::Arc, time::Instant};
use tokio::task::JoinSet;
type Completion = Result<(usize, NativeOutput, Instant), InstanceError>;
#[derive(Clone)]
struct Context {
    dispatch: InstanceDispatcher,
    registry: Arc<Registry>,
    apply: &'static CStr,
    admitted: Instant,
}
fn spawn(
    pending: &mut JoinSet<Completion>,
    context: &Context,
    index: usize,
    owner: usize,
    input: Input,
) {
    let context = context.clone();
    pending.spawn(async move {
        if context.admitted.elapsed() >= GATE {
            return Err(InstanceError::Timeout);
        }
        context
            .dispatch
            .submit_native_async_on(
                owner,
                prepare_named(
                    Command::Call(input),
                    context.registry,
                    context.apply,
                    owner,
                    "batch-call",
                ),
            )?
            .wait_completed(GATE.saturating_sub(context.admitted.elapsed()))
            .await
            .map(|(value, completed)| (index, value, completed))
    });
}
impl InstanceService {
    pub(super) fn pipeline(
        &self,
        inputs: &[Input],
        limit: usize,
    ) -> Answer<(Vec<Output>, Vec<u64>)> {
        let runtime = self.runtime()?;
        let first = self.reserve_owners(inputs.len());
        let mut outputs = Vec::new();
        let mut timings = Vec::new();
        let mut failure = None;
        for (group, inputs) in inputs.chunks(128).enumerate() {
            let admitted = Instant::now();
            let context = Context {
                dispatch: self.host.dispatch.clone(),
                registry: Arc::clone(&self.host.registry),
                apply: self.host.apply,
                admitted,
            };
            let results = runtime.block_on(async {
                let mut pending = JoinSet::new();
                let mut next = 0;
                let mut failed = false;
                let mut results = Vec::new();
                while next < inputs.len().min(limit) {
                    let owner = (first + group * 128 + next) % self.owners();
                    spawn(&mut pending, &context, next, owner, inputs[next].clone());
                    next += 1;
                }
                while let Some(result) = pending.join_next().await {
                    let result = result.unwrap_or(Err(InstanceError::TaskPanicked));
                    failed |= result.is_err();
                    results.push(result);
                    if !failed && next < inputs.len() {
                        if admitted.elapsed() >= GATE {
                            failed = true;
                            results.push(Err(InstanceError::Timeout));
                        } else {
                            let owner = (first + group * 128 + next) % self.owners();
                            spawn(&mut pending, &context, next, owner, inputs[next].clone());
                            next += 1;
                        }
                    }
                }
                results
            });
            self.host_span("rust-pipeline-group", admitted);
            let mut results = results;
            results
                .sort_by_key(|result| result.as_ref().map_or(usize::MAX, |(index, _, _)| *index));
            // Drain every accepted future before retiring any published partial result.
            for result in results {
                match result {
                    Ok((_, value, completed)) => match self.publish(value) {
                        Ok(value) => {
                            outputs.push(value);
                            timings.push(
                                u64::try_from(completed.duration_since(admitted).as_nanos())
                                    .unwrap_or(u64::MAX),
                            );
                        }
                        Err(error) => {
                            failure.get_or_insert(error);
                        }
                    },
                    Err(error) => {
                        failure.get_or_insert(error.to_string());
                    }
                }
            }
            if failure.is_some() {
                break;
            }
        }
        if let Some(error) = failure {
            for output in outputs {
                if let Output::Token(id) = output {
                    let _ = self.request(Command::Release(id));
                }
            }
            Err(error)
        } else {
            Ok((outputs, timings))
        }
    }
}
