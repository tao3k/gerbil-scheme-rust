//! Materialize all Python results before optional owner-batched retirement.
use super::{
    Arc, Bound, Ordering, Py, PyAny, PyList, PyResult, PyRuntimeError, PyTypeError, PyValueError,
    Python, PythonLibrary, PythonValue,
};
use pyo3::types::{PyAnyMethods, PyListMethods};

fn values(
    library: &PythonLibrary,
    py: Python<'_>,
    input: &Bound<'_, PyList>,
) -> PyResult<Vec<Py<PythonValue>>> {
    library
        .service
        .check_process()
        .map_err(PyRuntimeError::new_err)?;
    if input.len() > 4096 {
        return Err(PyValueError::new_err("native value batch exceeds 4096"));
    }
    input
        .iter()
        .map(|value| {
            let value = value
                .extract::<Py<PythonValue>>()
                .map_err(|_| PyTypeError::new_err("native Scheme values required"))?;
            if !Arc::ptr_eq(&library.service, &value.borrow(py).service) {
                return Err(PyValueError::new_err("foreign Scheme library"));
            }
            Ok(value)
        })
        .collect()
}
fn retire_tokens(py: Python<'_>, values: &[Py<PythonValue>], ids: &[u64]) {
    for (value, id) in values.iter().zip(ids) {
        let _ =
            value
                .borrow(py)
                .token
                .compare_exchange(*id, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}
pub(super) fn project_many(
    library: &PythonLibrary,
    py: Python<'_>,
    input: &Bound<'_, PyList>,
    release: bool,
) -> PyResult<Vec<Py<PyAny>>> {
    let values = values(library, py, input)?;
    let ids = values
        .iter()
        .map(|v| v.borrow(py).id())
        .collect::<PyResult<Vec<_>>>()?;
    let instances = library
        .service
        .instances
        .as_ref()
        .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
    let projected = py
        .detach(|| instances.project_many(&ids))
        .map_err(PyRuntimeError::new_err)?;
    let projected = projected
        .into_iter()
        .map(|value| value.to_python(py))
        .collect::<PyResult<Vec<_>>>()?;
    // Neither native projection failure nor Python allocation failure consumes input roots.
    if release {
        py.detach(|| instances.release_many(&ids))
            .map_err(PyRuntimeError::new_err)?;
        retire_tokens(py, &values, &ids);
    }
    Ok(projected)
}
pub(super) fn release_many(
    library: &PythonLibrary,
    py: Python<'_>,
    input: &Bound<'_, PyList>,
) -> PyResult<()> {
    let values = values(library, py, input)?;
    let ids = values
        .iter()
        .map(|value| value.borrow(py).token.load(Ordering::Acquire))
        .collect::<Vec<_>>();
    let instances = library
        .service
        .instances
        .as_ref()
        .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
    py.detach(|| instances.release_many(&ids))
        .map_err(PyRuntimeError::new_err)?;
    retire_tokens(py, &values, &ids);
    Ok(())
}
