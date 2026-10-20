//! Typed projection of a frozen native retirement observation.
use gerbil_scheme::instances::{ImageMappingReceipt, ImageMappingState};
use pyo3::{PyResult, exceptions::PyRuntimeError, pyclass};
use std::mem::ManuallyDrop;

#[pyclass(name = "ImageMappingState", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PythonImageMappingState {
    OwnerLive,
    RetainedByPolicy,
    RetainedByLeases,
    Quarantined,
    Unloading,
    Released,
    ReleaseFailed,
}
/// A receipt is an observation at close, not a lease or permission to enter Scheme.
#[pyclass(name = "ImageMappingReceipt", frozen)]
#[derive(Clone, Debug)]
pub struct PythonImageMappingReceipt {
    #[pyo3(get)]
    owner: usize,
    #[pyo3(get)]
    generation: u64,
    #[pyo3(get)]
    state: PythonImageMappingState,
    #[pyo3(get)]
    code_leases: usize,
    #[pyo3(get)]
    pending_module_closes: Option<u64>,
    #[pyo3(get)]
    error: Option<String>,
}
impl From<ImageMappingReceipt> for PythonImageMappingReceipt {
    fn from(receipt: ImageMappingReceipt) -> Self {
        let (state, error) = match receipt.state {
            ImageMappingState::OwnerLive => (PythonImageMappingState::OwnerLive, None),
            ImageMappingState::RetainedByPolicy => {
                (PythonImageMappingState::RetainedByPolicy, None)
            }
            ImageMappingState::RetainedByLeases(_) => {
                (PythonImageMappingState::RetainedByLeases, None)
            }
            ImageMappingState::Quarantined => (PythonImageMappingState::Quarantined, None),
            ImageMappingState::Unloading => (PythonImageMappingState::Unloading, None),
            ImageMappingState::Released => (PythonImageMappingState::Released, None),
            ImageMappingState::ReleaseFailed(error) => {
                (PythonImageMappingState::ReleaseFailed, Some(error))
            }
        };
        Self {
            owner: receipt.owner,
            generation: receipt.generation,
            state,
            code_leases: receipt.code_leases,
            pending_module_closes: receipt.pending_module_closes,
            error,
        }
    }
}

/// Host code ownership, distinct from a Scheme value or a live VM root.
#[pyclass(name = "ImageCodeLease", frozen)]
#[derive(Debug)]
pub struct PythonImageCodeLease {
    lease: ManuallyDrop<std::sync::Mutex<Option<gerbil_scheme::instances::ImageCodeLease>>>,
    pid: u32,
    retired: std::sync::Mutex<Option<ImageMappingReceipt>>,
}
impl PythonImageCodeLease {
    fn check_process(&self) -> PyResult<()> {
        if self.pid != std::process::id() {
            return Err(PyRuntimeError::new_err(
                "inherited native code lease after fork",
            ));
        }
        Ok(())
    }
    pub(super) fn new(lease: gerbil_scheme::instances::ImageCodeLease) -> Self {
        Self {
            lease: ManuallyDrop::new(std::sync::Mutex::new(Some(lease))),
            pid: std::process::id(),
            retired: std::sync::Mutex::new(None),
        }
    }
}
#[pyo3::pymethods]
impl PythonImageCodeLease {
    fn close(&self, py: pyo3::Python<'_>) -> PyResult<()> {
        self.check_process()?;
        let lease = self
            .lease
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(lease) = lease {
            let receipt = py.detach(|| lease.release());
            *self
                .retired
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(receipt);
        }
        Ok(())
    }
    #[getter]
    fn mapping(&self) -> PyResult<Option<PythonImageMappingReceipt>> {
        self.check_process()?;
        Ok(self
            .lease
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(gerbil_scheme::instances::ImageCodeLease::mapping)
            .or_else(|| {
                self.retired
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone()
            })
            .map(Into::into))
    }
}
impl Drop for PythonImageCodeLease {
    fn drop(&mut self) {
        // A forked child has no owner threads and may inherit locked mutexes.
        // Keep its private copy without acquiring locks or running unload hooks.
        if self.pid != std::process::id() {
            return;
        }
        let mut lease = self
            .lease
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        // Unload hooks run without the GIL. During interpreter finalization,
        // preserve code rather than calling native hooks through a dead host.
        if pyo3::Python::try_attach(|py| py.detach(|| drop(lease.take()))).is_none() {
            std::mem::forget(lease);
        }
        // SAFETY: same process, exclusive ownership and the guard was taken.
        unsafe { ManuallyDrop::drop(&mut self.lease) };
    }
}
