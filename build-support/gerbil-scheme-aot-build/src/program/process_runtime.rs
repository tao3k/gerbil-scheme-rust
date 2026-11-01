//! One shared, scoped Tokio runtime for concurrent synchronous build callers.
use std::sync::{Arc, Mutex, Weak};
use tokio::runtime::{Builder, Runtime};

static ACTIVE: Mutex<Weak<CompilerRuntime>> = Mutex::new(Weak::new());

pub(super) struct CompilerRuntime {
    runtime: Runtime,
}

impl CompilerRuntime {
    /// Owns the host-sized worker pool; no task is detached by process execution.
    fn build_runtime() -> Result<Self, String> {
        let workers = std::thread::available_parallelism()
            .map_err(|error| format!("read compiler host parallelism: {error}"))?
            .get();
        Ok(Self {
            runtime: Builder::new_multi_thread()
                .worker_threads(workers)
                .thread_name("gerbil-build")
                .enable_all()
                .build()
                .map_err(|error| format!("create compiler Tokio runtime: {error}"))?,
        })
    }

    pub(super) fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    #[cfg(test)]
    pub(super) fn worker_count(&self) -> usize {
        self.runtime.metrics().num_workers()
    }
}

pub(super) fn acquire() -> Result<Arc<CompilerRuntime>, String> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err("synchronous compiler entry cannot nest a Tokio runtime".into());
    }
    let mut active = ACTIVE
        .lock()
        .map_err(|_| "compiler runtime ownership poisoned")?;
    if let Some(runtime) = active.upgrade() {
        return Ok(runtime);
    }
    let runtime = Arc::new(CompilerRuntime::build_runtime()?);
    *active = Arc::downgrade(&runtime);
    Ok(runtime)
}
