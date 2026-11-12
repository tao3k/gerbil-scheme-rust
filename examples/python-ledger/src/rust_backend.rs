// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Both qualification policies dispatch to the same complete native Scheme image.
use gerbil_scheme_python::{
    PackageManifest, PythonInstanceProgram, PythonInstanceRequest, PythonLibrary,
    open_instance_library,
};
use pyo3::{PyResult, Python};
use std::path::Path;
fn program(workers: usize) -> PythonInstanceProgram {
    // SAFETY: canonical build-owned rooted exports and explicit Actor finalizer.
    unsafe {
        PythonInstanceProgram::new(
            "_resources/native/ledger.dylib",
            c"gerbil_python_ledger_call",
            c"gerbil_python_ledger_configure",
            c"gerbil_scheme_python_apply",
        )
        .with_finalizer(c"gerbil_python_ledger_close")
        .with_dispatch_workers(workers)
    }
}
fn admit(
    py: Python<'_>,
    root: &Path,
    manifest: &PackageManifest,
    owners: usize,
    program: PythonInstanceProgram,
) -> PyResult<PythonLibrary> {
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
pub fn open(
    py: Python<'_>,
    root: &Path,
    manifest: &PackageManifest,
    owners: usize,
    workers: usize,
) -> PyResult<PythonLibrary> {
    admit(py, root, manifest, owners, program(workers))
}
#[cfg(feature = "qualified-image-retirement")]
pub fn open_qualified(
    py: Python<'_>,
    root: &Path,
    manifest: &PackageManifest,
    owners: usize,
    workers: usize,
) -> PyResult<PythonLibrary> {
    // SAFETY: explicit controlled-workload qualification provider. Its callers
    // quiesce all VM callbacks and lease every escaping native-code reference.
    // Native extensions must satisfy that contract; no Scheme feature is disabled.
    let program = unsafe { program(workers).with_unload_after_join() };
    admit(py, root, manifest, owners, program)
}
