//! Explicit owner affinity; sessions on the same owner share its Scheme state.
use super::{
    Arc, BTreeSet, Bound, Command, PyAny, PyResult, PyRuntimeError, Python, PythonValue, Service,
    read, token,
};
#[cfg(feature = "instance-backend")]
use pyo3::types::{PyList, PyListMethods};
use pyo3::{pyclass, pymethods};

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
    fn call_many(
        &self,
        py: Python<'_>,
        requests: &Bound<'_, PyList>,
    ) -> PyResult<Vec<PythonValue>> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let inputs = requests
            .iter()
            .map(|v| read(&v, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let outputs = py
            .detach(|| instances.call_many_on(self.owner, &inputs))
            .map_err(PyRuntimeError::new_err)?;
        outputs
            .iter()
            .map(|output| token(&self.service, output))
            .collect()
    }
}
