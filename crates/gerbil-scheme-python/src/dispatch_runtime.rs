//! Own the Tokio executor; native Scheme work remains on independent fixed owners.
use std::future::Future;
use tokio::{runtime::Runtime, task::JoinHandle};
#[derive(Debug)]
pub(super) struct DispatchRuntime {
    runtime: Runtime,
}
impl DispatchRuntime {
    pub(super) fn start_runtime(workers: usize) -> std::io::Result<Self> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(workers)
            .thread_name("gerbil-dispatch")
            .enable_all()
            .build()
            .map(|runtime| Self { runtime })
    }
    pub(super) fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
    /// The caller stores and joins every returned handle before releasing this owner.
    pub(super) fn spawn_task<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
}
