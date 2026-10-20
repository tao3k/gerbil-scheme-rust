//! Matched direct and Tokio admission to the same fixed Scheme owners.
use super::super::Input;
use super::trace::prepare_named;
use super::{Answer, Command, GATE, InstanceError, InstanceService, Output};
use rayon::prelude::*;
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
    ) -> Answer<Vec<Output>> {
        self.call_many_routed(inputs, "direct", Some(owner), 128)
            .map(|(values, _)| values)
    }
    fn validate_batch(&self, inputs: &[Input], mode: &str, pinned: Option<usize>) -> Answer<()> {
        if pinned.is_some_and(|owner| owner >= self.owners()) {
            return Err("native owner index out of range".into());
        }
        if pinned.is_none() && inputs.iter().any(Input::contains_reference) {
            return Err("stateless requests cannot contain owner-bound Scheme references".into());
        }
        if !matches!(mode, "tokio" | "direct" | "rayon" | "rayon-blocking") {
            return Err("dispatch must be tokio, direct, rayon or rayon-blocking".into());
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
            let values = if mode == "direct" {
                self.direct_window(jobs, admitted)
            } else if mode == "rayon" || mode == "rayon-blocking" {
                self.rayon_window(jobs, admitted, mode == "rayon-blocking")?
            } else {
                let dispatch = self.host.dispatch.clone();
                let registry = Arc::clone(&self.host.registry);
                let apply = self.host.apply;
                runtime.block_on(async {
                    let mut pending = Vec::new();
                    for (owner, input) in jobs {
                        let dispatch = dispatch.clone();
                        let registry = Arc::clone(&registry);
                        pending.push(tokio::spawn(async move {
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
            };
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
    fn direct_window(
        &self,
        jobs: Vec<(usize, Input)>,
        admitted: Instant,
    ) -> Vec<Result<(super::NativeOutput, Instant), InstanceError>> {
        let mut pending = Vec::new();
        for (owner, input) in jobs {
            let registry = Arc::clone(&self.host.registry);
            let apply = self.host.apply;
            pending.push(self.host.dispatch.submit_native_on(
                owner,
                prepare_named(Command::Call(input), registry, apply, owner, "batch-call"),
            ));
        }
        pending
            .into_iter()
            .map(|ticket| {
                ticket.and_then(|ticket| {
                    ticket.wait_completed(GATE.saturating_sub(admitted.elapsed()))
                })
            })
            .collect::<Vec<_>>()
    }
    fn rayon_pool(&self) -> Answer<Arc<rayon::ThreadPool>> {
        let mut pool = self
            .host
            .rayon
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
