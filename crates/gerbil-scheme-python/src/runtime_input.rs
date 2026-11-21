//! Python native input conversion; no Scheme execution or scheduling.
#[cfg(feature = "instance-backend")]
use super::PythonPreparedRequest;
use super::{
    Arc, BTreeSet, Bound, Input, PyAny, PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyRef,
    PyResult, PyString, PyTuple, PyTypeError, PyValueError, PythonValue, Service,
};
use pyo3::types::{PyAnyMethods, PyBytesMethods, PyDictMethods, PyListMethods, PyTupleMethods};
pub(super) fn read(
    value: &Bound<'_, PyAny>,
    service: &Arc<Service>,
    seen: &mut BTreeSet<usize>,
) -> PyResult<Input> {
    if let Ok(root) = value.extract::<PyRef<'_, PythonValue>>() {
        if !Arc::ptr_eq(service, &root.service) {
            return Err(PyValueError::new_err("foreign Scheme owner"));
        }
        return Ok(Input::Reference(root.id()?));
    }
    if value.is_none() {
        return Ok(Input::Void);
    }
    if value.is_instance_of::<PyBool>() {
        return value.extract().map(Input::Bool);
    }
    if value.is_instance_of::<PyInt>() {
        return value.extract().map(Input::Integer);
    }
    if value.is_instance_of::<PyFloat>() {
        return value.extract().map(Input::Real);
    }
    if value.is_instance_of::<PyString>() {
        return value.extract().map(Input::Text);
    }
    if let Ok(bytes) = value.cast::<PyBytes>() {
        return Ok(Input::Bytes(bytes.as_bytes().to_vec()));
    }
    #[cfg(feature = "instance-backend")]
    if let Ok(request) = value.extract::<PyRef<'_, PythonPreparedRequest>>() {
        if seen.len() + request.depth > 128 {
            return Err(PyValueError::new_err(
                "prepared nesting exceeds bridge stack bound",
            ));
        }
        return Ok(Input::Shared(Arc::clone(&request.input)));
    }
    let address = value.as_ptr() as usize;
    if !seen.insert(address) {
        return Err(PyValueError::new_err(
            "cyclic Python containers require native handles",
        ));
    }
    // Bound Rust stack use independently of user fact/request capacities.
    if seen.len() > 128 {
        return Err(PyValueError::new_err(
            "Python container nesting exceeds bridge stack bound",
        ));
    }
    let result = if let Ok(values) = value.cast::<PyList>() {
        values
            .iter()
            .map(|v| read(&v, service, seen))
            .collect::<PyResult<_>>()
            .map(Input::List)
    } else if let Ok(values) = value.cast::<PyTuple>() {
        values
            .iter()
            .map(|v| read(&v, service, seen))
            .collect::<PyResult<_>>()
            .map(Input::Vector)
    } else if let Ok(values) = value.cast::<PyDict>() {
        values
            .iter()
            .map(|(k, v)| Ok((read(&k, service, seen)?, read(&v, service, seen)?)))
            .collect::<PyResult<_>>()
            .map(Input::Table)
    } else {
        Err(PyTypeError::new_err(
            "unsupported Python value; use a native handle",
        ))
    };
    seen.remove(&address);
    result
}
