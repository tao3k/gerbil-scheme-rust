// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Library-specific Rust code only describes its compiler-owned export.
#[cfg(feature = "rust-backend")]
use crate::rust_backend;
#[cfg(feature = "scheme-backend")]
use gerbil_scheme::LinkedGerbilProgram;
use gerbil_scheme_python::{PackageManifest, select_backend};
use gerbil_scheme_python::{PythonLibrary, PythonSession, PythonValue};
#[cfg(feature = "scheme-backend")]
use gerbil_scheme_python::{PythonProgram, open_library};
use pyo3::IntoPyObjectExt;
use pyo3::types::{PyModule, PyModuleMethods};
use pyo3::{Bound, Py, PyAny, PyResult, Python, pyfunction, pymodule, wrap_pyfunction};
use std::path::Path;

type Open = for<'py> fn(Python<'py>, &str, usize, usize) -> PyResult<Py<PyAny>>;

#[cfg(feature = "scheme-backend")]
unsafe extern "C" {
    fn ___LNK_python__ledger(
        gs: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn gerbil_python_ledger_call(value: i64) -> i64;
    fn gerbil_scheme_python_configure(value: i64) -> i64;
    fn gerbil_scheme_python_apply(value: i64) -> i64;
}
#[pyfunction]
#[pyo3(signature=(root, backend="rust", owners=4, dispatch_workers=2))]
fn _open(
    py: Python<'_>,
    root: &str,
    backend: &str,
    owners: usize,
    dispatch_workers: usize,
) -> PyResult<Py<PyAny>> {
    #[cfg(feature = "scheme-backend")]
    let scheme = Some(open_scheme as Open);
    #[cfg(not(feature = "scheme-backend"))]
    let scheme: Option<Open> = None;
    #[cfg(feature = "rust-backend")]
    let rust = Some(open_rust as Open);
    #[cfg(not(feature = "rust-backend"))]
    let rust: Option<Open> = None;
    let factory = select_backend(backend, scheme, rust)
        .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?;
    factory(py, root, owners, dispatch_workers)
}

fn manifest() -> PyResult<PackageManifest> {
    PackageManifest::from_json(include_bytes!(concat!(env!("OUT_DIR"), "/package.json")))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[cfg(feature = "rust-backend")]
fn open_rust(
    py: Python<'_>,
    root: &str,
    owners: usize,
    dispatch_workers: usize,
) -> PyResult<Py<PyAny>> {
    let manifest = manifest()?;
    rust_backend::open(py, Path::new(root), &manifest, owners, dispatch_workers)?.into_py_any(py)
}

#[cfg(feature = "scheme-backend")]
fn open_scheme(
    py: Python<'_>,
    root: &str,
    _owners: usize,
    _dispatch_workers: usize,
) -> PyResult<Py<PyAny>> {
    let manifest = manifest()?;
    // SAFETY: build.rs consumes the official compiler-owned graph with one
    // upstream ABI module and these process-lived rooted exports.
    let program = unsafe {
        PythonProgram::new(
            LinkedGerbilProgram::from_linker(___LNK_python__ledger),
            gerbil_python_ledger_call,
            gerbil_scheme_python_configure,
            gerbil_scheme_python_apply,
        )
    };
    open_library(py, program, &manifest, Path::new(root))?.into_py_any(py)
}
#[pyfunction]
fn available_backends() -> Vec<&'static str> {
    [
        ("scheme", cfg!(feature = "scheme-backend")),
        ("rust", cfg!(feature = "rust-backend")),
    ]
    .into_iter()
    .filter_map(|(name, compiled)| compiled.then_some(name))
    .collect()
}

#[pyfunction]
fn scheme_initialization_attempted() -> Option<bool> {
    #[cfg(feature = "rust-backend")]
    {
        Some(gerbil_scheme::instances::InstancePool::has_been_initialized())
    }
    #[cfg(not(feature = "rust-backend"))]
    {
        Some(gerbil_scheme::GerbilRuntime::has_been_initialized())
    }
}

#[cfg(feature = "qualified-image-retirement")]
#[pyfunction]
#[pyo3(signature=(root, owners=4, dispatch_workers=2))]
fn _open_qualified_retirement(
    py: Python<'_>,
    root: &str,
    owners: usize,
    dispatch_workers: usize,
) -> PyResult<Py<PyAny>> {
    rust_backend::open_qualified(py, Path::new(root), &manifest()?, owners, dispatch_workers)?
        .into_py_any(py)
}
#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    {
        #[cfg(feature = "rust-backend")]
        module.add_class::<gerbil_scheme_python::PythonPreparedRequest>()?;
        #[cfg(feature = "rust-backend")]
        module.add_class::<gerbil_scheme_python::PythonImageMappingState>()?;
        #[cfg(feature = "rust-backend")]
        module.add_class::<gerbil_scheme_python::PythonImageCodeLease>()?;
        #[cfg(feature = "rust-backend")]
        module.add_class::<gerbil_scheme_python::PythonImageMappingReceipt>()?;
        module.add_class::<PythonLibrary>()?;
        module.add_class::<PythonValue>()?;
        module.add_class::<PythonSession>()?;
    }
    #[cfg(feature = "qualified-image-retirement")]
    module.add_function(wrap_pyfunction!(_open_qualified_retirement, module)?)?;
    module.add_function(wrap_pyfunction!(available_backends, module)?)?;
    module.add_function(wrap_pyfunction!(scheme_initialization_attempted, module)?)?;
    module.add_function(wrap_pyfunction!(_open, module)?)
}
