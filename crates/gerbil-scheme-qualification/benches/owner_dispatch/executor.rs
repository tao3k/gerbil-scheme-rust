//! Executors dispatch across owners; no executor worker enters a Scheme VM.
use super::GATE;
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_json::json;
use std::{sync::mpsc, time::Instant};
pub(super) type Completed = Result<(Vec<Vec<i64>>, Instant), InstanceError>;
pub(super) type Job = (usize, Vec<Vec<i64>>);
pub(super) enum Engine {
    Direct,
    Tokio(tokio::runtime::Runtime),
    Rayon(rayon::ThreadPool),
}
pub(super) struct Executor {
    pub(super) engine: Engine,
    pub profile: bool,
    pub grouped: bool,
    pub name: &'static str,
    pub workers: usize,
    pub setup_ns: u128,
    starts: Vec<String>,
    stopped: mpsc::Receiver<String>,
}
impl Executor {
    pub(super) fn start(name: &'static str, workers: usize, profile: bool, grouped: bool) -> Self {
        let started = Instant::now();
        let (ready, starts) = mpsc::channel();
        let (exit, stopped) = mpsc::channel();
        let engine = match name {
            "direct" => {
                assert_eq!(workers, 0);
                Engine::Direct
            }
            "tokio" | "owner-flow" => Engine::Tokio(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(workers)
                    .enable_all()
                    .thread_name("owner-bench-tokio")
                    .on_thread_start(move || {
                        ready
                            .send(format!("{:?}", std::thread::current().id()))
                            .unwrap();
                    })
                    .on_thread_stop(move || {
                        let _ = exit.send(format!("{:?}", std::thread::current().id()));
                    })
                    .build()
                    .unwrap(),
            ),
            "rayon" => Engine::Rayon(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(workers)
                    .thread_name(|n| format!("owner-bench-rayon-{n}"))
                    .start_handler(move |_| {
                        ready
                            .send(format!("{:?}", std::thread::current().id()))
                            .unwrap();
                    })
                    .exit_handler(move |_| {
                        let _ = exit.send(format!("{:?}", std::thread::current().id()));
                    })
                    .build()
                    .unwrap(),
            ),
            _ => panic!("unknown dispatcher"),
        };
        let mut ids: Vec<_> = (0..workers)
            .map(|_| starts.recv_timeout(GATE).unwrap())
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), workers);
        Self {
            engine,
            profile,
            grouped,
            name,
            workers,
            setup_ns: started.elapsed().as_nanos(),
            starts: ids,
            stopped,
        }
    }
    pub(super) fn window(
        &self,
        dispatch: &InstanceDispatcher,
        jobs: Vec<Job>,
        released: Instant,
    ) -> Vec<Completed> {
        if self.profile {
            return super::attribution::window(self, dispatch, jobs, released);
        }
        if self.name == "owner-flow" {
            let Engine::Tokio(runtime) = &self.engine else {
                unreachable!()
            };
            return runtime.block_on(gerbil_scheme_python::owner_flow::owner_flow_window(
                jobs,
                gerbil_scheme_python::owner_flow::FlowControl {
                    budget: gerbil_scheme_python::owner_dispatch::WindowBudget {
                        released,
                        timeout: GATE,
                    },
                    maximum_in_flight: 128,
                },
                |owner, items| dispatch.submit_numeric_batch_async_on(owner, items, 1),
            ));
        }
        if self.grouped && self.name != "direct" {
            return self.grouped_window(dispatch, jobs, released);
        }
        let remaining = || GATE.saturating_sub(released.elapsed());
        let submit = |(owner, items)| {
            if released.elapsed() >= GATE {
                return Err(InstanceError::Timeout);
            }
            dispatch.submit_numeric_batch_on(owner, items, 1)
        };
        match &self.engine {
            Engine::Direct => {
                let tickets: Vec<_> = jobs.into_iter().map(submit).collect();
                tickets
                    .into_iter()
                    .map(|t| t.and_then(|t| t.wait_completed(remaining())))
                    .collect()
            }
            Engine::Rayon(pool) => {
                // Submission is parallel; native waiting stays on the caller.
                // No worker is occupied waiting on the Scheme owner queue.
                let tickets: Vec<_> = pool.install(|| jobs.into_par_iter().map(submit).collect());
                tickets
                    .into_iter()
                    .map(|t| t.and_then(|t| t.wait_completed(remaining())))
                    .collect()
            }
            Engine::Tokio(runtime) => runtime.block_on(async {
                let tasks: Vec<_> = jobs
                    .into_iter()
                    .map(|(owner, items)| {
                        let dispatch = dispatch.clone();
                        runtime.spawn(async move {
                            if released.elapsed() >= GATE {
                                return Err(InstanceError::Timeout);
                            }
                            dispatch
                                .submit_numeric_batch_async_on(owner, items, 1)?
                                .wait_completed(GATE.saturating_sub(released.elapsed()))
                                .await
                        })
                    })
                    .collect();
                let mut results = Vec::new();
                for task in tasks {
                    results.push(task.await.unwrap_or(Err(InstanceError::TaskPanicked)));
                }
                results
            }),
        }
    }
    fn grouped_window(
        &self,
        dispatch: &InstanceDispatcher,
        jobs: Vec<Job>,
        released: Instant,
    ) -> Vec<Completed> {
        match &self.engine {
            Engine::Tokio(runtime) => {
                let dispatch = dispatch.clone();
                runtime.block_on(gerbil_scheme_python::owner_dispatch::tokio_owner_window(
                    jobs,
                    released,
                    GATE,
                    move |owner, items| dispatch.submit_numeric_batch_async_on(owner, items, 1),
                ))
            }
            Engine::Rayon(pool) => {
                let tickets = gerbil_scheme_python::owner_dispatch::rayon_owner_submit(
                    pool,
                    jobs,
                    |owner, items| {
                        if released.elapsed() >= GATE {
                            return Err(InstanceError::Timeout);
                        }
                        dispatch.submit_numeric_batch_on(owner, items, 1)
                    },
                );
                tickets
                    .into_iter()
                    .map(|t| {
                        t.and_then(|t| t.wait_completed(GATE.saturating_sub(released.elapsed())))
                    })
                    .collect()
            }
            Engine::Direct => unreachable!("direct A/A uses the unchanged direct window"),
        }
    }
    pub(super) fn finish(self) {
        let Self {
            engine,
            name,
            workers,
            starts,
            stopped,
            setup_ns,
            profile: _,
            grouped: _,
        } = self;
        let started = Instant::now();
        drop(engine);
        let mut exits: Vec<_> = (0..workers)
            .map(|_| stopped.recv_timeout(GATE).unwrap())
            .collect();
        exits.sort();
        assert_eq!(starts, exits);
        println!(
            "{}",
            json!({"event": "dispatch_executor", "mode": name, "workers": workers, "started_thread_ids": starts, "stopped_thread_ids": exits, "setup_ns": setup_ns, "shutdown_ns": started.elapsed().as_nanos()})
        );
    }
}
