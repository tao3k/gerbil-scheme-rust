//! Explicit owner affinity; sessions on the same owner share its Scheme state.
use super::{
    Arc, BTreeSet, Bound, Command, PyAny, PyResult, PyRuntimeError, Python, PythonValue, Service,
    read, token,
};
#[cfg(feature = "instance-backend")]
use pyo3::types::{PyList, PyListMethods};
use pyo3::{pyclass, pymethods};

/// Explicit affinity handle sharing the selected owner's native Scheme state.
#[pyclass(module = "gerbil_scheme", frozen)]
#[derive(Debug)]
pub struct PythonSession {
    service: Arc<Service>,
    owner: usize,
}
impl PythonSession {
    pub(super) fn new(service: &Arc<Service>, owner: usize) -> Self {
        Self {
            service: Arc::clone(service),
            owner,
        }
    }
    fn invoke(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        construct: bool,
    ) -> PyResult<PythonValue> {
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let command = if construct {
            Command::Encode(input)
        } else {
            Command::Call(input)
        };
        let output = py
            .detach(|| self.service.request_on(self.owner, command))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
}
#[pymethods]
impl PythonSession {
    #[getter]
    fn owner(&self) -> usize {
        self.owner
    }
    fn call(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        self.invoke(py, value, false)
    }
    fn value(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        self.invoke(py, value, true)
    }
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (requests, *, dispatch="direct", admission_window=128))]
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
        let read_started = self
            .service
            .instances
            .as_ref()
            .and_then(super::instances::InstanceService::profile_start);
        let inputs = requests
            .iter()
            .map(|v| read(&v, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        instances.profile_finish("python-session-read-many", read_started);
        let (outputs, returned) = py.detach(|| {
            let outputs = instances.call_many_on(self.owner, &inputs, dispatch, admission_window);
            (outputs, instances.profile_start())
        });
        // Includes reacquisition, scheduling and the PyO3 return boundary.
        instances.profile_finish("gil-return-session", returned);
        let outputs = outputs.map_err(PyRuntimeError::new_err)?;
        let wrapping = instances.profile_start();
        let result = outputs
            .iter()
            .map(|output| token(&self.service, output))
            .collect();
        instances.profile_finish("python-session-wrap-results", wrapping);
        result
    }
}
