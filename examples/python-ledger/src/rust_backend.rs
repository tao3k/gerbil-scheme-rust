// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Rust dispatch owns no ledger algorithm; each fixed owner runs native Scheme.
use gerbil_scheme_python::{
    PackageManifest, PythonInstanceProgram, PythonInstanceRequest, PythonLibrary,
    open_instance_library,
};
use pyo3::{PyResult, Python};
use std::path::Path;
pub fn open(
    py: Python<'_>,
    root: &Path,
    manifest: &PackageManifest,
    owners: usize,
    dispatch_workers: usize,
) -> PyResult<PythonLibrary> {
    // SAFETY: the canonical Gerbil build owns these rooted exports and packages
    // the private runtime image under the verified installed resource manifest.
    let program = unsafe {
        PythonInstanceProgram::new(
            "_resources/native/ledger.dylib",
            c"gerbil_python_ledger_call",
            c"gerbil_python_ledger_configure",
            c"gerbil_scheme_python_apply",
        )
        .with_finalizer(c"gerbil_python_ledger_close")
        .with_dispatch_workers(dispatch_workers)
    };
    open_instance_library(
        py,
        PythonInstanceRequest {
            program,
            manifest,
            root,
            owners,
        },
    )
}
