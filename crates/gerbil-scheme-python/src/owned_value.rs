// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! VM-independent owned transport values shared by Rust execution providers.
use crate::{OwnedValue, ValueLimits};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    exceptions::PyValueError,
    types::{
        PyAnyMethods, PyBool, PyBytes, PyBytesMethods, PyDict, PyDictMethods, PyFloat, PyInt,
        PyList, PyListMethods, PyString, PyTuple, PyTupleMethods,
    },
};
use std::collections::BTreeMap;

impl OwnedValue {
    /// Convert a resource ceiling without accepting Python booleans as integers.
    /// # Errors
    /// Rejects non-integers, zero/negative values and host word overflow.
    pub fn positive_usize(&self) -> PyResult<usize> {
        let Self::Integer(value) = self else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "positive integer ceiling required",
            ));
        };
        if value.sign() != num_bigint::Sign::Plus || value.bits() > u64::from(usize::BITS) {
            return Err(PyValueError::new_err(
                "positive host-sized integer ceiling required",
            ));
        }
        let (_, words) = value.to_u64_digits();
        usize::try_from(words[0]).map_err(|_| PyValueError::new_err("integer ceiling overflow"))
    }
    /// Read Python data without starting a runtime or interpreting source text.
    /// # Errors
    /// Rejects unsupported objects, non-string map keys and exhausted bounds.
    pub fn from_python(value: &Bound<'_, PyAny>, limits: ValueLimits) -> PyResult<Self> {
        Decoder { limits, nodes: 0 }.read(value, 0)
    }
    /// Copy owned values to Python; no external resource or VM is consulted.
    /// # Errors
    /// Propagates Python allocation/conversion errors.
    pub fn to_python(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Self::Null => Ok(py.None()),
            Self::Boolean(v) => v.into_py_any(py),
            Self::Integer(v) => v.into_py_any(py),
            Self::Real(v) => v.into_py_any(py),
            Self::Text(v) => v.into_py_any(py),
            Self::Bytes(v) => Ok(PyBytes::new(py, v).unbind().into_any()),
            Self::List(values) => write_list(py, values),
            Self::Map(values) => write_map(py, values),
        }
    }
}
struct Decoder {
    limits: ValueLimits,
    nodes: usize,
}
impl Decoder {
    fn read(&mut self, value: &Bound<'_, PyAny>, depth: usize) -> PyResult<OwnedValue> {
        if depth > self.limits.depth || self.nodes >= self.limits.nodes {
            return Err(PyValueError::new_err("owned value structure bound"));
        }
        self.nodes += 1;
        if value.is_none() {
            return Ok(OwnedValue::Null);
        }
        if value.is_instance_of::<PyBool>() {
            return value.extract().map(OwnedValue::Boolean);
        }
        if value.is_instance_of::<PyInt>() {
            return value.extract().map(OwnedValue::Integer);
        }
        if value.is_instance_of::<PyFloat>() {
            return value.extract().map(OwnedValue::Real);
        }
        if value.is_instance_of::<PyString>() {
            return value.extract().map(OwnedValue::Text);
        }
        if let Ok(bytes) = value.cast::<PyBytes>() {
            return Ok(OwnedValue::Bytes(bytes.as_bytes().to_vec()));
        }
        if let Ok(values) = value.cast::<PyList>() {
            return values
                .iter()
                .map(|v| self.read(&v, depth + 1))
                .collect::<PyResult<_>>()
                .map(OwnedValue::List);
        }
        if let Ok(values) = value.cast::<PyTuple>() {
            return values
                .iter()
                .map(|v| self.read(&v, depth + 1))
                .collect::<PyResult<_>>()
                .map(OwnedValue::List);
        }
        if let Ok(values) = value.cast::<PyDict>() {
            return self.map(values, depth);
        }
        Err(PyValueError::new_err(
            "unsupported owned value; explicit provider handle required",
        ))
    }
    fn map(&mut self, values: &Bound<'_, PyDict>, depth: usize) -> PyResult<OwnedValue> {
        values
            .iter()
            .map(|(key, value)| Ok((key.extract::<String>()?, self.read(&value, depth + 1)?)))
            .collect::<PyResult<_>>()
            .map(OwnedValue::Map)
    }
}
fn write_list(py: Python<'_>, values: &[OwnedValue]) -> PyResult<Py<PyAny>> {
    let out = PyList::empty(py);
    for value in values {
        out.append(value.to_python(py)?)?;
    }
    Ok(out.unbind().into_any())
}
fn write_map(py: Python<'_>, values: &BTreeMap<String, OwnedValue>) -> PyResult<Py<PyAny>> {
    let out = PyDict::new(py);
    for (name, value) in values {
        out.set_item(name, value.to_python(py)?)?;
    }
    Ok(out.unbind().into_any())
}
