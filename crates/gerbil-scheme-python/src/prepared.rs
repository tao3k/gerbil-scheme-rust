//! Immutable portable host snapshots. Every invocation still constructs native Scheme data.
use super::{Arc, BTreeSet, Bound, Input, PyAny, PyResult, PyValueError, PythonLibrary, read};
use pyo3::pyclass;
/// An owned typed request snapshot without Scheme references or Python objects.
#[derive(Debug)]
#[pyclass(name = "PreparedRequest", frozen)]
pub struct PythonPreparedRequest {
    pub(super) input: Arc<Input>,
    pub(super) depth: usize,
}
fn depth(input: &Input) -> usize {
    match input {
        Input::Shared(value) => depth(value),
        Input::List(values) | Input::Vector(values) => {
            1 + values.iter().map(depth).max().unwrap_or(0)
        }
        Input::Table(values) => {
            1 + values
                .iter()
                .map(|(k, v)| depth(k).max(depth(v)))
                .max()
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// Snapshot portable typed input once; no native object, compilation or VM entry occurs.
pub(super) fn prepare(
    library: &PythonLibrary,
    value: &Bound<'_, PyAny>,
) -> PyResult<PythonPreparedRequest> {
    library
        .service
        .check_process()
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
    let input = read(value, &library.service, &mut BTreeSet::new())?;
    if input.contains_reference() {
        return Err(PyValueError::new_err(
            "prepared requests cannot contain Scheme references",
        ));
    }
    let depth = depth(&input);
    let input = if let Input::Shared(input) = input {
        input
    } else {
        Arc::new(input)
    };
    Ok(PythonPreparedRequest { input, depth })
}
