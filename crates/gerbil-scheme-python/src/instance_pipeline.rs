//! Completion-driven bounded admission; every Scheme operation stays on its fixed owner.
use super::{
    Answer, Command, GATE, InstanceError, InstanceService, NativeOutput, Output, Registry,
    trace::prepare_named,
};
use crate::runtime::Input;
use gerbil_scheme::instances::InstanceDispatcher;
use std::{collections::VecDeque, ffi::CStr, sync::Arc, time::Instant};
use tokio::task::JoinSet;
type NativeResult = Result<(usize, NativeOutput, Instant), InstanceError>;
type Completion = (usize, NativeResult);
#[derive(Clone)]
struct Context {
    dispatch: InstanceDispatcher,
    registry: Arc<Registry>,
    apply: &'static CStr,
    admitted: Instant,
}
fn admit_one(
    pending: &mut JoinSet<Completion>,
    context: &Context,
    index: usize,
    owner: usize,
    input: Input,
) {
    let context = context.clone();
    pending.spawn(async move {
        let result = async move {
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
        }
        .await;
        (owner, result)
    });
}
struct Schedule {
    waiting: Vec<VecDeque<usize>>,
    active: Vec<usize>,
    cursor: usize,
    quota: usize,
    remaining: usize,
}
impl Schedule {
    fn new(owners: usize, first: usize, count: usize, limit: usize) -> Self {
        let mut waiting = vec![VecDeque::new(); owners];
        for index in 0..count {
            waiting[(first + index) % owners].push_back(index);
        }
        Self {
            waiting,
            active: vec![0; owners],
            cursor: first % owners,
            quota: limit.div_ceil(owners),
            remaining: count,
        }
    }
    fn admit_next(&mut self) -> Option<(usize, usize)> {
        let owners = self.waiting.len();
        for offset in 0..owners {
            let owner = (self.cursor + offset) % owners;
            if self.active[owner] < self.quota {
                if let Some(index) = self.waiting[owner].pop_front() {
                    self.active[owner] += 1;
                    self.remaining -= 1;
                    self.cursor = (owner + 1) % owners;
                    return Some((index, owner));
                }
            }
        }
        None
    }
}
fn fill(
    pending: &mut JoinSet<Completion>,
    context: &Context,
    schedule: &mut Schedule,
    inputs: &[Input],
    limit: usize,
) {
    while pending.len() < limit && context.admitted.elapsed() < GATE {
        let Some((index, owner)) = schedule.admit_next() else {
            break;
        };
        admit_one(pending, context, index, owner, inputs[index].clone());
    }
}

async fn collect_group(
    context: &Context,
    inputs: &[Input],
    owners: usize,
    first: usize,
    limit: usize,
) -> Vec<NativeResult> {
    let mut pending = JoinSet::new();
    let mut schedule = Schedule::new(owners, first, inputs.len(), limit);
    let mut failed = false;
    let mut results = Vec::new();
    loop {
        if !failed {
            fill(&mut pending, context, &mut schedule, inputs, limit);
            if schedule.remaining > 0 && context.admitted.elapsed() >= GATE {
                failed = true;
                results.push(Err(InstanceError::Timeout));
            }
        }
        let Some(completed) = pending.join_next().await else {
            break;
        };
        let result = match completed {
            Ok((owner, result)) => {
                schedule.active[owner] -= 1;
                result
            }
            Err(_) => Err(InstanceError::TaskPanicked),
        };
        failed |= result.is_err();
        results.push(result);
    }
    results
}

struct PublishedGroup {
    outputs: Vec<Output>,
    timings: Vec<u64>,
    failure: Option<String>,
}
impl InstanceService {
    fn publish_group(&self, results: Vec<NativeResult>, admitted: Instant) -> PublishedGroup {
        let mut outputs = Vec::new();
        let mut timings = Vec::new();
        let mut failure = None;
        let mut results = results;
        results.sort_by_key(|result| result.as_ref().map_or(usize::MAX, |(index, _, _)| *index));
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
        PublishedGroup {
            outputs,
            timings,
            failure,
        }
    }

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
            let results = runtime.block_on(collect_group(
                &context,
                inputs,
                self.owners(),
                first + group * 128,
                limit,
            ));
            self.host_span("rust-pipeline-group", admitted);
            let published = self.publish_group(results, admitted);
            outputs.extend(published.outputs);
            timings.extend(published.timings);
            failure = published.failure;
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
