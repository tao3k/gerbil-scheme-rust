//! Matched direct and Tokio admission to the same fixed Scheme owners.
use super::trace::prepare_named;
use super::{Answer, Command, GATE, InstanceError, InstanceService, Output};
use crate::runtime::Input;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Arc, time::Instant};
impl InstanceService {
    pub(in super::super) fn call_many(
        &self,
        inputs: &[Input],
        mode: &str,
        admission_window: usize,
    ) -> Answer<Vec<Output>> {
        self.call_many_profiled(inputs, mode, admission_window)
            .map(|(outputs, _)| outputs)
    }
    pub(in super::super) fn call_many_profiled(
        &self,
        inputs: &[Input],
        mode: &str,
        admission_window: usize,
    ) -> Answer<(Vec<Output>, Vec<u64>)> {
        self.call_many_routed(inputs, mode, None, admission_window)
    }
    pub(in super::super) fn call_many_on(
        &self,
        owner: usize,
        inputs: &[Input],
        mode: &str,
        admission_window: usize,
    ) -> Answer<Vec<Output>> {
        self.call_many_routed(inputs, mode, Some(owner), admission_window)
            .map(|(values, _)| values)
    }
    fn validate_batch(&self, inputs: &[Input], mode: &str, pinned: Option<usize>) -> Answer<()> {
        if pinned.is_some() && mode == "tokio-pipeline" {
            return Err("tokio-pipeline requires stateless routing".into());
        }
        if pinned.is_some_and(|owner| owner >= self.owners()) {
            return Err("native owner index out of range".into());
        }
        if pinned.is_none() && inputs.iter().any(Input::contains_reference) {
            return Err("stateless requests cannot contain owner-bound Scheme references".into());
        }
        if !matches!(
            mode,
            "tokio" | "direct" | "rayon" | "rayon-blocking" | "tokio-pipeline"
        ) {
            return Err(
                "dispatch must be tokio, direct, rayon, rayon-blocking or tokio-pipeline".into(),
            );
        }
        if inputs.len() > 4096 {
            return Err("native request batch exceeds 4096".into());
        }
        if let Some(owner) = pinned {
            for input in inputs {
                self.validate_input_owner(input, owner)?;
            }
        }
        Ok(())
    }
    fn call_many_routed(
        &self,
        inputs: &[Input],
        mode: &str,
        pinned: Option<usize>,
        admission_window: usize,
    ) -> Answer<(Vec<Output>, Vec<u64>)> {
        self.validate_batch(inputs, mode, pinned)?;
        if !(1..=128).contains(&admission_window) {
            return Err("admission_window must be in 1..=128".into());
        }
        if mode == "tokio-pipeline" {
            return self.pipeline(inputs, admission_window);
        }
        let runtime = self.runtime()?;
        let first_owner = if pinned.is_some() {
            0
        } else {
            self.reserve_owners(inputs.len())
        };
        let mut outputs = Vec::new();
        let mut timings = Vec::new();
        let mut error = None;
        // Smaller turns share the original 128-request deadline and timing origin.
        let windows = inputs.chunks(128).enumerate().flat_map(|(group, inputs)| {
            inputs
                .chunks(admission_window)
                .enumerate()
                .map(move |(turn, inputs)| (group * 128 + turn * admission_window, inputs))
        });
        let mut admitted = Instant::now();
        for (offset, window) in windows {
            if offset % 128 == 0 {
                admitted = Instant::now();
            }
            if admitted.elapsed() >= GATE {
                error = Some(InstanceError::Timeout.to_string());
                break;
            }
            let turn_started = Instant::now();
            let jobs = window
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, input)| {
                    (
                        pinned.unwrap_or((first_owner + offset + index) % self.owners()),
                        input,
                    )
                })
                .collect::<Vec<_>>();
            self.host_span("rust-clone-turn", turn_started);
            let values = if pinned.is_some() && mode != "direct" {
                self.pinned_window(&runtime, jobs, admitted, mode)?
            } else if mode == "direct" {
                self.direct_window(jobs, admitted)
            } else if mode == "rayon" || mode == "rayon-blocking" {
                self.rayon_window(jobs, admitted, mode == "rayon-blocking")?
            } else {
                self.tokio_window(&runtime, jobs, admitted)
            };
            error = self.publish_window(values, admitted, &mut outputs, &mut timings);
            self.host_span("rust-admission-turn", turn_started);
            if error.is_some() {
                break;
            }
        }
        if let Some(error) = error {
            for output in outputs {
                if let Output::Token(id) = output {
                    let _ = self.request(Command::Release(id));
                }
            }
            return Err(error);
        }
        Ok((outputs, timings))
    }
}

impl InstanceService {
    fn publish_window(
        &self,
        values: Vec<Result<(super::NativeOutput, Instant), InstanceError>>,
        admitted: Instant,
        outputs: &mut Vec<Output>,
        timings: &mut Vec<u64>,
    ) -> Option<String> {
        let mut error = None;
        for value in values {
            match value.and_then(|(value, completed)| {
                self.publish(value)
                    .map(|value| (value, completed))
                    .map_err(InstanceError::Loader)
            }) {
                Ok((value, completed)) => {
                    outputs.push(value);
                    timings.push(
                        u64::try_from(completed.duration_since(admitted).as_nanos())
                            .unwrap_or(u64::MAX),
                    );
                }
                Err(failure) => {
                    error.get_or_insert(failure.to_string());
                }
            }
        }
        error
    }
    fn tokio_window(
        &self,
        runtime: &super::DispatchRuntime,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
    ) -> Vec<Result<(super::NativeOutput, Instant), InstanceError>> {
        let dispatch = self.host.dispatch.clone();
        let registry = Arc::clone(&self.host.registry);
        let apply = self.host.apply;
        runtime.block_on(async {
            let mut pending = Vec::new();
            for (owner, input) in jobs {
                let dispatch = dispatch.clone();
                let registry = Arc::clone(&registry);
                pending.push(runtime.spawn_task(async move {
                    if admitted.elapsed() >= GATE {
                        return Err(InstanceError::Timeout);
                    }
                    dispatch
                        .submit_native_async_on(
                            owner,
                            prepare_named(
                                Command::Call(input),
                                registry,
                                apply,
                                owner,
                                "batch-call",
                            ),
                        )?
                        .wait_completed(GATE.saturating_sub(admitted.elapsed()))
                        .await
                }));
            }
            let mut results = Vec::new();
            for job in pending {
                results.push(job.await.unwrap_or(Err(InstanceError::TaskPanicked)));
            }
            results
        })
    }
    fn submit_ordered(
        &self,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
    ) -> Vec<Result<gerbil_scheme::instances::InstanceTicket<super::NativeOutput>, InstanceError>>
    {
        jobs.into_iter()
            .map(|(owner, input)| {
                if admitted.elapsed() >= GATE {
                    return Err(InstanceError::Timeout);
                }
                self.host.dispatch.submit_native_on(
                    owner,
                    prepare_named(
                        Command::Call(input),
                        Arc::clone(&self.host.registry),
                        self.host.apply,
                        owner,
                        "batch-call",
                    ),
                )
            })
            .collect()
    }
    fn direct_window(
        &self,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
    ) -> Vec<Result<(super::NativeOutput, Instant), InstanceError>> {
        self.submit_ordered(jobs, admitted)
            .into_iter()
            .map(|ticket| {
                ticket.and_then(|ticket| {
                    ticket.wait_completed(GATE.saturating_sub(admitted.elapsed()))
                })
            })
            .collect()
    }
    // Stateful owner streams admit in input order. The scheduler may observe
    // replies concurrently, but cannot reorder Scheme state transitions.
    fn pinned_window(
        &self,
        runtime: &super::DispatchRuntime,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
        mode: &str,
    ) -> Answer<Vec<Result<(super::NativeOutput, Instant), InstanceError>>> {
        if mode == "tokio" {
            let tickets = jobs
                .into_iter()
                .map(|(owner, input)| {
                    if admitted.elapsed() >= GATE {
                        return Err(InstanceError::Timeout);
                    }
                    self.host.dispatch.submit_native_async_on(
                        owner,
                        prepare_named(
                            Command::Call(input),
                            Arc::clone(&self.host.registry),
                            self.host.apply,
                            owner,
                            "batch-call",
                        ),
                    )
                })
                .collect::<Vec<_>>();
            return Ok(runtime.block_on(async {
                let pending = tickets
                    .into_iter()
                    .map(|ticket| {
                        runtime.spawn_task(async move {
                            ticket?
                                .wait_completed(GATE.saturating_sub(admitted.elapsed()))
                                .await
                        })
                    })
                    .collect::<Vec<_>>();
                let mut results = Vec::new();
                for task in pending {
                    results.push(task.await.unwrap_or(Err(InstanceError::TaskPanicked)));
                }
                results
            }));
        }
        let pool = self.rayon_pool()?;
        if mode == "rayon-blocking" {
            return Ok(pool.install(|| self.direct_window(jobs, admitted)));
        }
        let tickets = pool.install(|| self.submit_ordered(jobs, admitted));
        Ok(tickets
            .into_iter()
            .map(|ticket| {
                ticket.and_then(|ticket| {
                    ticket.wait_completed(GATE.saturating_sub(admitted.elapsed()))
                })
            })
            .collect())
    }
    fn rayon_pool(&self) -> Answer<Arc<rayon::ThreadPool>> {
        let mut pool = self
            .host
            .rayon
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Close sets the flag before taking this same pool lock. No late
        // caller may recreate the executor after shutdown has retired it.
        if self.host.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(InstanceError::Closed.to_string());
        }
        if let Some(pool) = pool.as_ref() {
            return Ok(Arc::clone(pool));
        }
        let created = rayon::ThreadPoolBuilder::new()
            .num_threads(self.host.workers)
            .thread_name(|n| format!("gerbil-dispatch-rayon-{n}"))
            .build()
            .map_err(|e| e.to_string())?;
        let created = Arc::new(created);
        *pool = Some(Arc::clone(&created));
        Ok(created)
    }
    fn rayon_window(
        &self,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
        blocking: bool,
    ) -> Answer<Vec<Result<(super::NativeOutput, Instant), InstanceError>>> {
        let pool = self.rayon_pool()?;
        let registry = Arc::clone(&self.host.registry);
        let apply = self.host.apply;
        let submit = |(owner, input)| {
            if admitted.elapsed() >= GATE {
                return Err(InstanceError::Timeout);
            }
            let registry = Arc::clone(&registry);
            self.host.dispatch.submit_native_on(
                owner,
                prepare_named(Command::Call(input), registry, apply, owner, "batch-call"),
            )
        };
        let wait = |ticket: Result<
            gerbil_scheme::instances::InstanceTicket<super::NativeOutput>,
            InstanceError,
        >| {
            ticket.and_then(|ticket| ticket.wait_completed(GATE.saturating_sub(admitted.elapsed())))
        };
        if blocking {
            return Ok(pool.install(|| jobs.into_par_iter().map(|job| wait(submit(job))).collect()));
        }
        let tickets = pool.install(|| jobs.into_par_iter().map(submit).collect::<Vec<_>>());
        Ok(tickets.into_iter().map(wait).collect())
    }
}
