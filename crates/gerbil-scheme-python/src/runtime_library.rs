//! Python library operations dispatch to the selected fixed Scheme owners.
use super::{
    BTreeSet, Bound, Command, ExecutionKind, GerbilRuntime, Output, PyAny, PyRef, PyResult,
    PyRuntimeError, PyValueError, Python, PythonLibrary, PythonRuntimeCapabilities, PythonSession,
    PythonValue, VmCapabilities, protocol, pymethods, read, token,
};
#[cfg(feature = "instance-backend")]
use super::{Py, PyList, PythonPreparedRequest, instances, prepared, projection_api};
#[cfg(feature = "instance-backend")]
use pyo3::types::PyListMethods;
#[pymethods]
impl PythonLibrary {
    #[getter]
    fn backend(&self) -> &'static str {
        match self.service.kind {
            ExecutionKind::Linked => "scheme",
            #[cfg(feature = "instance-backend")]
            ExecutionKind::Independent(_) => "rust",
        }
    }
    /// Lifecycle observation without Scheme VM entry or SDK configuration.
    #[staticmethod]
    fn scheme_initialization_attempted() -> bool {
        #[cfg(feature = "instance-backend")]
        if gerbil_scheme::instances::InstancePool::has_been_initialized() {
            return true;
        }
        GerbilRuntime::has_been_initialized()
    }
    /// Observe actual active processors on this library's owner, not SDK capacity.
    #[getter]
    fn active_processors(&self, py: Python<'_>) -> PyResult<u32> {
        match py
            .detach(|| self.service.request(Command::ProcessorCount))
            .map_err(PyRuntimeError::new_err)?
        {
            Output::ProcessorCount(count) => Ok(count),
            _ => Err(protocol()),
        }
    }
    /// Inspect the linked ABI without initializing a runtime or reading SDK
    /// environment variables. Tokio tasks and Python threads do not add VMs.
    #[staticmethod]
    fn capabilities() -> PythonRuntimeCapabilities {
        PythonRuntimeCapabilities {
            linked: VmCapabilities::linked(),
            model: if cfg!(feature = "instance-backend") {
                "available-fixed-scheme-owners"
            } else {
                "process-global-single-vm-owner"
            },
            independent: cfg!(all(feature = "instance-backend", target_os = "macos")),
        }
    }
    /// Explicit audit boundary; retained calls do not repeatedly scan immutable resources.
    fn verify_resources(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.service.package.reverify())
            .map(|_| ())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
    /// Root Python input as an actual Scheme object, not a JSON scalar envelope.
    fn value(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let output = py
            .detach(|| self.service.request(Command::Encode(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// A session binds calls and constructed values to one VM; sessions on one owner share its Scheme state.
    fn session(&self, owner: usize) -> PyResult<PythonSession> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        if owner >= self.vm_count() {
            return Err(PyValueError::new_err("native owner index out of range"));
        }
        Ok(PythonSession::new(&self.service, owner))
    }
    /// Caller-declared independent work; live native references require a session.
    #[cfg(feature = "instance-backend")]
    fn call_stateless(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let output = py
            .detach(|| instances.call_stateless(input))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// Project native values in owner batches. Projection errors preserve every root.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (values, *, release=false))]
    fn project_many(
        &self,
        py: Python<'_>,
        values: &Bound<'_, PyList>,
        release: bool,
    ) -> PyResult<Vec<Py<PyAny>>> {
        projection_api::project_many(self, py, values, release)
    }
    /// Idempotent owner-batched release. A delivery failure may release a subset; retry is safe.
    #[cfg(feature = "instance-backend")]
    fn release_many(&self, py: Python<'_>, values: &Bound<'_, PyList>) -> PyResult<()> {
        projection_api::release_many(self, py, values)
    }
    /// Invoke the default stateful session on owner 0 with the GIL released.
    fn call(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let output = py
            .detach(|| self.service.request(Command::Call(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    #[getter]
    fn vm_count(&self) -> usize {
        match self.service.kind {
            ExecutionKind::Linked => 1,
            #[cfg(feature = "instance-backend")]
            ExecutionKind::Independent(owners) => owners,
        }
    }
    #[getter]
    fn execution_model(&self) -> &'static str {
        match self.service.kind {
            ExecutionKind::Linked => "process-global-single-vm-owner",
            #[cfg(feature = "instance-backend")]
            ExecutionKind::Independent(_) => "tokio-dispatch-fixed-scheme-owners",
        }
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn vm_states(&self) -> Vec<usize> {
        self.service
            .instances
            .as_ref()
            .map_or_else(Vec::new, instances::InstanceService::states)
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn shutdown_clean(&self) -> Option<bool> {
        self.service
            .instances
            .as_ref()
            .and_then(instances::InstanceService::shutdown_receipt)
            .map(|r| {
                r.is_clean()
                    && self
                        .service
                        .instances
                        .as_ref()
                        .is_some_and(instances::InstanceService::resources_removed)
            })
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn shutdown_roots(&self) -> Vec<i64> {
        self.service
            .instances
            .as_ref()
            .and_then(instances::InstanceService::shutdown_receipt)
            .map_or_else(Vec::new, |r| {
                r.owners.iter().map(|o| o.roots_before_cleanup).collect()
            })
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn runtime_directory(&self) -> Option<String> {
        self.service
            .instances
            .as_ref()
            .map(instances::InstanceService::resource_directory)
    }
    #[cfg(feature = "instance-backend")]
    fn collect(&self, py: Python<'_>) -> PyResult<Vec<[f64; 4]>> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        py.detach(|| instances.collect())
            .map_err(PyRuntimeError::new_err)
    }
    /// Pin a stateful session to an explicit native owner; handles remain owner checked.
    #[cfg(feature = "instance-backend")]
    fn call_on(
        &self,
        py: Python<'_>,
        owner: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PythonValue> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let output = py
            .detach(|| instances.request_on(owner, Command::Call(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// Snapshot portable typed input once without native entry or compilation.
    #[cfg(feature = "instance-backend")]
    fn prepare_request(&self, value: &Bound<'_, PyAny>) -> PyResult<PythonPreparedRequest> {
        prepared::prepare(self, value)
    }
    /// Independent requests are assigned across instances; stateful call stays on owner 0.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (requests, *, dispatch="tokio", admission_window=128))]
    fn call_many(
        &self,
        py: Python<'_>,
        requests: &Bound<'_, PyList>,
        dispatch: &str,
        admission_window: usize,
    ) -> PyResult<Vec<PythonValue>> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let read_started = std::time::Instant::now();
        let inputs = requests
            .iter()
            .map(|v| read(&v, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-read-many", read_started);
        let (values, detached) = py.detach(|| {
            let value = instances.call_many(&inputs, dispatch, admission_window);
            (value, std::time::Instant::now())
        });
        instances.host_span("gil-reacquire", detached);
        let values = values.map_err(PyRuntimeError::new_err)?;
        let materialize = std::time::Instant::now();
        let result = values
            .into_iter()
            .map(|output| token(&self.service, &output))
            .collect();
        instances.host_span("python-wrap-results", materialize);
        result
    }
    /// Enable bounded diagnostic spans. Toggle only after outstanding work drains.
    #[cfg(feature = "instance-backend")]
    fn set_owner_profiling(&self, enabled: bool) -> PyResult<()> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        self.service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?
            .set_owner_profiling(enabled)
            .map_err(PyRuntimeError::new_err)
    }
    /// Drain owned JSON diagnostics; request and result values still use native types.
    #[cfg(feature = "instance-backend")]
    fn owner_profile(&self) -> PyResult<String> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        self.service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?
            .owner_profile()
            .map_err(PyRuntimeError::new_err)
    }
    /// Return per-window admission-to-native-ready nanoseconds, before Python projection.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (requests, *, dispatch="tokio", admission_window=128))]
    fn call_many_profiled(
        &self,
        py: Python<'_>,
        requests: &Bound<'_, PyList>,
        dispatch: &str,
        admission_window: usize,
    ) -> PyResult<(Vec<PythonValue>, Vec<u64>)> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let read_started = std::time::Instant::now();
        let inputs = requests
            .iter()
            .map(|value| read(&value, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-read-many", read_started);
        let (result, detached) = py.detach(|| {
            let result = instances.call_many_profiled(&inputs, dispatch, admission_window);
            (result, std::time::Instant::now())
        });
        instances.host_span("gil-reacquire", detached);
        let (outputs, timings) = result.map_err(PyRuntimeError::new_err)?;
        let materialize = std::time::Instant::now();
        let values = outputs
            .into_iter()
            .map(|output| token(&self.service, &output))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-wrap-results", materialize);
        Ok((values, timings))
    }
    /// Release all roots and join the VM owner; no forced interruption is implied.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.service.close())
            .map_err(PyRuntimeError::new_err)
    }
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __exit__(
        &self,
        py: Python<'_>,
        _ty: &Bound<'_, PyAny>,
        _value: &Bound<'_, PyAny>,
        _trace: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close(py)
    }
}
