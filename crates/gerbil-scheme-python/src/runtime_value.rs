//! Python result handles retain owner affinity through projection and release.
use super::{
    Arc, BTreeSet, Bound, Command, Input, Ordering, Output, Projection, Py, PyAny, PyResult,
    PyRuntimeError, PyTuple, PyValueError, Python, PythonValue, protocol, pymethods, read, token,
};
use pyo3::types::PyTupleMethods;
#[pymethods]
impl PythonValue {
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<String> {
        match self.inspect(py, Projection::Kind)? {
            Output::Kind(v) => Ok(v),
            _ => Err(protocol()),
        }
    }
    /// Scalar projection is separate from native ownership/execution semantics.
    #[getter]
    fn owner(&self) -> PyResult<usize> {
        self.service
            .token_owner(self.id()?)
            .map_err(PyRuntimeError::new_err)
    }
    fn to_python(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.inspect(py, Projection::Scalar)? {
            Output::Scalar(value) => value.to_python(py),
            _ => Err(protocol()),
        }
    }
    /// Shallow native snapshot; shared/cyclic children remain native handles.
    fn elements(&self, py: Python<'_>) -> PyResult<Vec<Self>> {
        match self.inspect(py, Projection::Elements)? {
            Output::Tokens(values) => Ok(values
                .into_iter()
                .map(|v| Self::new(&self.service, v))
                .collect()),
            _ => Err(protocol()),
        }
    }
    #[pyo3(signature = (*arguments))]
    fn apply(&self, py: Python<'_>, arguments: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let id = self.id()?;
        let arguments = arguments
            .iter()
            .map(|v| read(&v, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        let output = py
            .detach(|| {
                self.service
                    .request(Command::Apply(id, Input::List(arguments)))
            })
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    fn is_identical(&self, py: Python<'_>, other: &Self) -> PyResult<bool> {
        if !Arc::ptr_eq(&self.service, &other.service) {
            return Err(PyValueError::new_err("foreign Scheme owner"));
        }
        let (a, b) = (self.id()?, other.id()?);
        match py
            .detach(|| self.service.request(Command::Identical(a, b)))
            .map_err(PyRuntimeError::new_err)?
        {
            Output::Bool(v) => Ok(v),
            _ => Err(protocol()),
        }
    }
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        let token = self.token.swap(0, Ordering::AcqRel);
        if token == 0 {
            return Ok(());
        }
        py.detach(|| self.service.request(Command::Release(token)))
            .map(|_| ())
            .map_err(PyRuntimeError::new_err)
    }
}
