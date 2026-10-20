// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Python owns opaque tokens; all native roots stay on the original VM thread.
use crate::{OwnedValue as Scalar, PackageManifest, VerifiedPackage};
use gerbil_scheme::{
    BinaryInteger, CollectionKind, GerbilRuntime, LinkedGerbilProgram, ObjectKind,
    RootedObjectBatch, RootedSchemeObject, SchemeScalar, TableEquality, VmCapabilities,
};
use num_bigint::{BigInt, Sign};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::types::{
    PyAnyMethods, PyBool, PyBytes, PyBytesMethods, PyDict, PyDictMethods, PyFloat, PyInt, PyList,
    PyListMethods, PyString, PyTuple, PyTupleMethods,
};
use pyo3::{Bound, Py, PyAny, PyErr, PyRef, PyResult, Python, pyclass, pymethods};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread::JoinHandle;

#[cfg(feature = "instance-backend")]
#[path = "instance_runtime.rs"]
mod instances;
#[cfg(feature = "instance-backend")]
pub use instances::{PythonInstanceProgram, open_instance_library};
#[path = "instance_resources.rs"]
mod resources;
#[path = "session.rs"]
mod session;
pub use session::PythonSession;
#[cfg(feature = "instance-backend")]
#[path = "prepared.rs"]
mod prepared;
#[cfg(feature = "instance-backend")]
#[path = "instance_projection_api.rs"]
mod projection_api;
#[cfg(feature = "instance-backend")]
pub use prepared::PythonPreparedRequest;
type Answer<T> = Result<T, String>;
type Root<'a> = RootedSchemeObject<'a>;

/// Library-maintainer-owned linked exports. No Python function accepts pointers,
/// compiler paths, module search paths or a runtime bootstrap source.
#[derive(Clone, Copy, Debug)]
pub struct PythonProgram {
    program: LinkedGerbilProgram,
    dispatch: unsafe extern "C" fn(i64) -> i64,
    configure: unsafe extern "C" fn(i64) -> i64,
    apply: unsafe extern "C" fn(i64) -> i64,
}
impl PythonProgram {
    /// Bind a compiler-owned graph and its strong-root C exports.
    /// # Safety
    /// All exports must satisfy `GerbilRuntime::bind_object_export`'s contract,
    /// belong to `program`, and live for the process. `configure` must accept a
    /// list of package roots and use the official runtime loader API; it must
    /// not load code from ambient SDK paths or compile user code.
    #[must_use]
    pub const unsafe fn new(
        program: LinkedGerbilProgram,
        dispatch: unsafe extern "C" fn(i64) -> i64,
        configure: unsafe extern "C" fn(i64) -> i64,
        apply: unsafe extern "C" fn(i64) -> i64,
    ) -> Self {
        Self {
            program,
            dispatch,
            configure,
            apply,
        }
    }
}

#[derive(Clone, Debug)]
enum Input {
    Void,
    Bool(bool),
    Integer(BigInt),
    Real(f64),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<Self>),
    Vector(Vec<Self>),
    Table(Vec<(Self, Self)>),
    Reference(u64),
    #[cfg(feature = "instance-backend")]
    Shared(Arc<Self>),
}
#[cfg(feature = "instance-backend")]
impl Input {
    fn contains_reference(&self) -> bool {
        match self {
            Self::Reference(_) => true,
            Self::List(values) | Self::Vector(values) => {
                values.iter().any(Self::contains_reference)
            }
            Self::Table(values) => values
                .iter()
                .any(|(key, value)| key.contains_reference() || value.contains_reference()),
            _ => false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum Projection {
    Kind,
    Scalar,
    Elements,
}
enum Command {
    ProcessorCount,
    Encode(Input),
    Call(Input),
    Apply(u64, Input),
    Identical(u64, u64),
    Inspect(u64, Projection),
    Release(u64),
    Close,
}
enum Output {
    ProcessorCount(u32),
    Token(u64),
    Kind(String),
    Scalar(Scalar),
    Tokens(Vec<u64>),
    Unit,
    Bool(bool),
}
struct Request {
    command: Command,
    reply: mpsc::SyncSender<Answer<Output>>,
}
#[derive(Debug)]
struct Connection {
    sender: mpsc::Sender<Request>,
    worker: JoinHandle<()>,
}
#[derive(Debug)]
struct Service {
    connection: Mutex<Option<Connection>>,
    pid: u32,
    package: VerifiedPackage,
    namespace: Option<std::path::PathBuf>,
    #[cfg(feature = "instance-backend")]
    instances: Option<instances::InstanceService>,
}

impl Service {
    fn check_process(&self) -> Answer<()> {
        if self.pid != std::process::id() {
            return Err("inherited Scheme runtime cannot enter after fork".into());
        }
        Ok(())
    }
    fn request_on(&self, owner: usize, command: Command) -> Answer<Output> {
        self.check_process()?;
        #[cfg(feature = "instance-backend")]
        if let Some(instances) = &self.instances {
            return instances.request_on(owner, command);
        }
        if owner != 0 {
            return Err("native owner index out of range".into());
        }
        self.request(command)
    }
    fn token_owner(&self, id: u64) -> Answer<usize> {
        self.check_process()?;
        #[cfg(feature = "instance-backend")]
        if let Some(instances) = &self.instances {
            return instances.token_owner(id);
        }
        let _ = id;
        Ok(0)
    }
    fn request(&self, command: Command) -> Answer<Output> {
        self.check_process()?;
        #[cfg(feature = "instance-backend")]
        if let Some(instances) = &self.instances {
            return instances.request(command);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Scheme owner lock poisoned")?;
        // A completed/ongoing explicit close owns bulk root retirement. Value
        // release remains idempotent; all other calls still reject that owner.
        if connection.is_none() && matches!(command, Command::Release(_)) {
            return Ok(Output::Unit);
        }
        let connection = connection.as_ref().ok_or("Scheme library is closed")?;
        let (reply, response) = mpsc::sync_channel(1);
        connection
            .sender
            .send(Request { command, reply })
            .map_err(|_| "Scheme owner terminated")?;
        response.recv().map_err(|_| "Scheme owner terminated")?
    }
    fn close(&self) -> Answer<()> {
        self.check_process()?;
        #[cfg(feature = "instance-backend")]
        if let Some(instances) = &self.instances {
            return instances.close();
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Scheme owner lock poisoned")?
            .take();
        if let Some(connection) = connection {
            let (reply, response) = mpsc::sync_channel(1);
            let sent = connection
                .sender
                .send(Request {
                    command: Command::Close,
                    reply,
                })
                .is_ok();
            drop(connection.sender);
            let result = if sent {
                response
                    .recv()
                    .map_err(|_| "Scheme owner terminated")
                    .and_then(|r| r.map_err(|_| "Scheme close failed"))
            } else {
                Err("Scheme owner terminated")
            };
            connection
                .worker
                .join()
                .map_err(|_| "Scheme owner panicked")?;
            result?;
            if let Some(namespace) = &self.namespace {
                resources::retire(namespace).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// One Python-visible library lifetime, using the upstream one-shot VM owner.
/// This is a synchronous C ABI bridge, not an Actor scheduler or a sandbox.
#[pyclass(module = "gerbil_scheme", frozen)]
#[derive(Debug)]
pub struct PythonLibrary {
    service: Arc<Service>,
}

/// Linked ABI observations and the execution topology actually implemented by
/// this bridge. Storage flags never grant independent VM execution admission.
#[pyclass(module = "gerbil_scheme", frozen)]
#[derive(Clone, Copy, Debug)]
pub struct PythonRuntimeCapabilities {
    linked: VmCapabilities,
    model: &'static str,
    independent: bool,
}

#[pymethods]
impl PythonRuntimeCapabilities {
    #[getter]
    fn multiple_vm_storage(&self) -> bool {
        self.linked.multiple_vms()
    }

    #[getter]
    fn thread_local_entry(&self) -> bool {
        self.linked.thread_local_entry()
    }

    /// Compiled capacity, not active processors or an independent VM count.
    #[getter]
    fn max_processors(&self) -> u32 {
        self.linked.max_processors()
    }

    #[getter]
    fn execution_model(&self) -> &'static str {
        self.model
    }
    #[getter]
    fn independent_vm_execution(&self) -> bool {
        self.independent
    }
}

/// A strong native result, including procedures, cyclic/shared objects and
/// objects with no Python projection. Projection failure does not release it.
#[pyclass(module = "gerbil_scheme", frozen)]
#[derive(Debug)]
pub struct PythonValue {
    service: Arc<Service>,
    token: AtomicU64,
}
impl PythonValue {
    fn new(service: &Arc<Service>, token: u64) -> Self {
        Self {
            service: service.clone(),
            token: AtomicU64::new(token),
        }
    }
    fn id(&self) -> PyResult<u64> {
        let token = self.token.load(Ordering::Acquire);
        if token == 0 {
            Err(PyRuntimeError::new_err("Scheme value is released"))
        } else {
            Ok(token)
        }
    }
    fn inspect(&self, py: Python<'_>, projection: Projection) -> PyResult<Output> {
        let token = self.id()?;
        py.detach(|| self.service.request(Command::Inspect(token, projection)))
            .map_err(PyRuntimeError::new_err)
    }
}
impl Drop for PythonValue {
    fn drop(&mut self) {
        let token = self.token.swap(0, Ordering::AcqRel);
        if token != 0 {
            let _ = self.service.request(Command::Release(token));
        }
    }
}

#[pymethods]
impl PythonLibrary {
    #[getter]
    fn backend(&self) -> &'static str {
        #[cfg(feature = "instance-backend")]
        if self.service.instances.is_some() {
            return "rust";
        }
        "scheme"
    }
    /// Lifecycle observation without Scheme VM entry or SDK configuration.
    #[staticmethod]
    fn scheme_initialization_attempted() -> bool {
        #[cfg(feature = "instance-backend")]
        if gerbil_scheme::instances::InstancePool::has_been_initialized() {
            return true;
        }
        GerbilRuntime::has_been_initialized()
    }
    /// Observe actual active processors on this library's owner, not SDK capacity.
    #[getter]
    fn active_processors(&self, py: Python<'_>) -> PyResult<u32> {
        match py
            .detach(|| self.service.request(Command::ProcessorCount))
            .map_err(PyRuntimeError::new_err)?
        {
            Output::ProcessorCount(count) => Ok(count),
            _ => Err(protocol()),
        }
    }
    /// Inspect the linked ABI without initializing a runtime or reading SDK
    /// environment variables. Tokio tasks and Python threads do not add VMs.
    #[staticmethod]
    fn capabilities() -> PythonRuntimeCapabilities {
        PythonRuntimeCapabilities {
            linked: VmCapabilities::linked(),
            model: if cfg!(feature = "instance-backend") {
                "available-fixed-scheme-owners"
            } else {
                "process-global-single-vm-owner"
            },
            independent: cfg!(all(feature = "instance-backend", target_os = "macos")),
        }
    }
    /// Explicit audit boundary; retained calls do not repeatedly scan immutable resources.
    fn verify_resources(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.service.package.reverify())
            .map(|_| ())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
    /// Root Python input as an actual Scheme object, not a JSON scalar envelope.
    fn value(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let output = py
            .detach(|| self.service.request(Command::Encode(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// A session binds calls and constructed values to one VM; sessions on one owner share its Scheme state.
    fn session(&self, owner: usize) -> PyResult<PythonSession> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        if owner >= self.vm_count() {
            return Err(PyValueError::new_err("native owner index out of range"));
        }
        Ok(PythonSession::new(&self.service, owner))
    }
    /// Caller-declared independent work; live native references require a session.
    #[cfg(feature = "instance-backend")]
    fn call_stateless(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let output = py
            .detach(|| instances.call_stateless(input))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// Project native values in owner batches. Projection errors preserve every root.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (values, *, release=false, fast_scalars=false))]
    fn project_many(
        &self,
        py: Python<'_>,
        values: &Bound<'_, PyList>,
        release: bool,
        fast_scalars: bool,
    ) -> PyResult<Vec<Py<PyAny>>> {
        projection_api::project_many(self, py, values, release, fast_scalars)
    }
    /// Idempotent owner-batched release. A delivery failure may release a subset; retry is safe.
    #[cfg(feature = "instance-backend")]
    fn release_many(&self, py: Python<'_>, values: &Bound<'_, PyList>) -> PyResult<()> {
        projection_api::release_many(self, py, values)
    }
    /// Invoke the default stateful session on owner 0 with the GIL released.
    fn call(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PythonValue> {
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let output = py
            .detach(|| self.service.request(Command::Call(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    #[getter]
    fn vm_count(&self) -> usize {
        #[cfg(feature = "instance-backend")]
        if let Some(instances) = &self.service.instances {
            return instances.owners();
        }
        1
    }
    #[getter]
    fn execution_model(&self) -> &'static str {
        #[cfg(feature = "instance-backend")]
        if self.service.instances.is_some() {
            return "tokio-dispatch-fixed-scheme-owners";
        }
        "process-global-single-vm-owner"
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn vm_states(&self) -> Vec<usize> {
        self.service
            .instances
            .as_ref()
            .map_or_else(Vec::new, instances::InstanceService::states)
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn shutdown_clean(&self) -> Option<bool> {
        self.service
            .instances
            .as_ref()
            .and_then(instances::InstanceService::shutdown_receipt)
            .map(|r| {
                r.is_clean()
                    && self
                        .service
                        .instances
                        .as_ref()
                        .is_some_and(instances::InstanceService::resources_removed)
            })
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn shutdown_roots(&self) -> Vec<i64> {
        self.service
            .instances
            .as_ref()
            .and_then(instances::InstanceService::shutdown_receipt)
            .map_or_else(Vec::new, |r| {
                r.owners.iter().map(|o| o.roots_before_cleanup).collect()
            })
    }
    #[cfg(feature = "instance-backend")]
    #[getter]
    fn runtime_directory(&self) -> Option<String> {
        self.service
            .instances
            .as_ref()
            .map(instances::InstanceService::resource_directory)
    }
    #[cfg(feature = "instance-backend")]
    fn collect(&self, py: Python<'_>) -> PyResult<Vec<[f64; 4]>> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        py.detach(|| instances.collect())
            .map_err(PyRuntimeError::new_err)
    }
    /// Pin a stateful session to an explicit native owner; handles remain owner checked.
    #[cfg(feature = "instance-backend")]
    fn call_on(
        &self,
        py: Python<'_>,
        owner: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PythonValue> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let input = read(value, &self.service, &mut BTreeSet::new())?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let output = py
            .detach(|| instances.request_on(owner, Command::Call(input)))
            .map_err(PyRuntimeError::new_err)?;
        token(&self.service, &output)
    }
    /// Snapshot portable typed input once without native entry or compilation.
    #[cfg(feature = "instance-backend")]
    fn prepare_request(&self, value: &Bound<'_, PyAny>) -> PyResult<PythonPreparedRequest> {
        prepared::prepare(self, value)
    }
    /// Independent requests are assigned across instances; stateful call stays on owner 0.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (requests, *, dispatch="tokio", admission_window=128))]
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
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let read_started = std::time::Instant::now();
        let inputs = requests
            .iter()
            .map(|v| read(&v, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-read-many", read_started);
        let (values, detached) = py.detach(|| {
            let value = instances.call_many(&inputs, dispatch, admission_window);
            (value, std::time::Instant::now())
        });
        instances.host_span("gil-reacquire", detached);
        let values = values.map_err(PyRuntimeError::new_err)?;
        let materialize = std::time::Instant::now();
        let result = values
            .into_iter()
            .map(|output| token(&self.service, &output))
            .collect();
        instances.host_span("python-wrap-results", materialize);
        result
    }
    /// Enable bounded diagnostic spans. Toggle only after outstanding work drains.
    #[cfg(feature = "instance-backend")]
    fn set_owner_profiling(&self, enabled: bool) -> PyResult<()> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        self.service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?
            .set_owner_profiling(enabled)
            .map_err(PyRuntimeError::new_err)
    }
    /// Drain owned JSON diagnostics; request and result values still use native types.
    #[cfg(feature = "instance-backend")]
    fn owner_profile(&self) -> PyResult<String> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        self.service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?
            .owner_profile()
            .map_err(PyRuntimeError::new_err)
    }
    /// Return per-window admission-to-native-ready nanoseconds, before Python projection.
    #[cfg(feature = "instance-backend")]
    #[pyo3(signature = (requests, *, dispatch="tokio", admission_window=128))]
    fn call_many_profiled(
        &self,
        py: Python<'_>,
        requests: &Bound<'_, PyList>,
        dispatch: &str,
        admission_window: usize,
    ) -> PyResult<(Vec<PythonValue>, Vec<u64>)> {
        self.service
            .check_process()
            .map_err(PyRuntimeError::new_err)?;
        let instances = self
            .service
            .instances
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("instance backend required"))?;
        let read_started = std::time::Instant::now();
        let inputs = requests
            .iter()
            .map(|value| read(&value, &self.service, &mut BTreeSet::new()))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-read-many", read_started);
        let (result, detached) = py.detach(|| {
            let result = instances.call_many_profiled(&inputs, dispatch, admission_window);
            (result, std::time::Instant::now())
        });
        instances.host_span("gil-reacquire", detached);
        let (outputs, timings) = result.map_err(PyRuntimeError::new_err)?;
        let materialize = std::time::Instant::now();
        let values = outputs
            .into_iter()
            .map(|output| token(&self.service, &output))
            .collect::<PyResult<Vec<_>>>()?;
        instances.host_span("python-wrap-results", materialize);
        Ok((values, timings))
    }
    /// Release all roots and join the VM owner; no forced interruption is implied.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| self.service.close())
            .map_err(PyRuntimeError::new_err)
    }
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __exit__(
        &self,
        py: Python<'_>,
        _ty: &Bound<'_, PyAny>,
        _value: &Bound<'_, PyAny>,
        _trace: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close(py)
    }
}
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
fn token(service: &Arc<Service>, output: &Output) -> PyResult<PythonValue> {
    match output {
        Output::Token(v) => Ok(PythonValue::new(service, *v)),
        _ => Err(protocol()),
    }
}
fn protocol() -> PyErr {
    PyRuntimeError::new_err("Scheme bridge response mismatch")
}

fn read(
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

/// Open a verified, statically linked Scheme Library. Only maintainers construct
/// `program`; Python receives no SDK configuration or compiler capability.
/// # Errors
/// Preserves complete package admission, one-shot lifecycle and native failures.
pub fn open_library(
    py: Python<'_>,
    program: PythonProgram,
    manifest: &PackageManifest,
    package_root: &Path,
) -> PyResult<PythonLibrary> {
    let service = py
        .detach(|| {
            // One complete admission, before runtime entry; no duplicate scan
            // in the facade and no stat-only substitute for per-path SHA.
            let package = manifest.verify(package_root).map_err(|e| e.to_string())?;
            let (paths, namespace) = resources::linked(&package).map_err(|e| e.to_string())?;
            let (sender, requests) = mpsc::channel();
            let (ready, initialized) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("gerbil-python-owner".into())
                .spawn(move || owner(program, &paths, requests, &ready))
                .map_err(|e| e.to_string())?;
            match initialized.recv() {
                Ok(Ok(())) => Ok(Arc::new(Service {
                    connection: Mutex::new(Some(Connection { sender, worker })),
                    pid: std::process::id(),
                    package,
                    namespace,
                    #[cfg(feature = "instance-backend")]
                    instances: None,
                })),
                Ok(Err(error)) => {
                    let _ = worker.join();
                    if let Some(namespace) = &namespace {
                        let _ = resources::retire(namespace);
                    }
                    Err(error)
                }
                Err(error) => {
                    let _ = worker.join();
                    if let Some(namespace) = &namespace {
                        let _ = resources::retire(namespace);
                    }
                    Err(error.to_string())
                }
            }
        })
        .map_err(PyRuntimeError::new_err)?;
    Ok(PythonLibrary { service })
}

fn configure_owner(
    runtime: &GerbilRuntime,
    program: PythonProgram,
    paths: &[String],
) -> Result<(), gerbil_scheme::NativeError> {
    (|| {
        let roots = paths
            .iter()
            .map(|p| {
                runtime
                    .string_from_utf8(p)
                    .map(gerbil_scheme::RootedSchemeString::into_object)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let input = runtime
            .collection_from_objects(CollectionKind::List, &roots.iter().collect::<Vec<_>>())?
            .into_object();
        // SAFETY: PythonProgram owns the library's checked linked configure export.
        unsafe { runtime.bind_object_export(program.configure) }?
            .call(&input)
            .into_result()
            .map(drop)
    })()
}

fn owner(
    program: PythonProgram,
    paths: &[String],
    requests: mpsc::Receiver<Request>,
    ready: &mpsc::SyncSender<Answer<()>>,
) {
    let runtime = match GerbilRuntime::initialize_program_single_processor(program.program) {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    let configured = configure_owner(&runtime, program, paths);
    if let Err(error) = configured {
        let _ = ready.send(Err(error.to_string()));
        return;
    }
    if ready.send(Ok(())).is_err() {
        return;
    }
    // SAFETY: PythonProgram requires this process-lived initialized export.
    let Ok(export) = (unsafe { runtime.bind_object_export(program.dispatch) }) else {
        return;
    };
    // SAFETY: the descriptor names the upstream linked rooted apply export.
    let Ok(apply) = (unsafe { runtime.bind_object_export(program.apply) }) else {
        return;
    };
    let mut roots: BTreeMap<u64, Root<'_>> = BTreeMap::new();
    let mut next = 1_u64;
    for request in requests {
        let closing = matches!(request.command, Command::Close);
        let calling = matches!(request.command, Command::Call(_));
        let result: Answer<Output> = (|| match request.command {
            Command::ProcessorCount => runtime
                .active_processors()
                .map(Output::ProcessorCount)
                .map_err(|e| e.to_string()),
            Command::Close => Ok(Output::Unit),
            Command::Release(id) => {
                roots.remove(&id);
                Ok(Output::Unit)
            }
            Command::Identical(a, b) => {
                let a = roots.get(&a).ok_or("Scheme value is released")?;
                let b = roots.get(&b).ok_or("Scheme value is released")?;
                Ok(Output::Bool(a.as_ref().is_identical(b.as_ref())))
            }
            Command::Apply(id, arguments) => {
                let procedure = roots.get(&id).ok_or("Scheme value is released")?;
                let arguments = encode(&runtime, &roots, &arguments).map_err(|e| e.to_string())?;
                let request = runtime
                    .collection_from_objects(CollectionKind::Vector, &[procedure, &arguments])
                    .map_err(|e| e.to_string())?
                    .into_object();
                let value = apply
                    .call(&request)
                    .into_result()
                    .map_err(|e| e.to_string())?;
                insert(&mut roots, &mut next, value).map(Output::Token)
            }
            Command::Encode(input) | Command::Call(input) => {
                let value = encode(&runtime, &roots, &input).map_err(|e| e.to_string())?;
                let value = if calling {
                    export
                        .call(&value)
                        .into_result()
                        .map_err(|e| e.to_string())?
                } else {
                    value
                };
                insert(&mut roots, &mut next, value).map(Output::Token)
            }
            Command::Inspect(id, projection) => {
                let root = roots.get(&id).ok_or("Scheme value is released")?;
                match projection {
                    Projection::Kind => root
                        .as_ref()
                        .kind()
                        .map(|k| Output::Kind(format!("{k:?}")))
                        .map_err(|e| e.to_string()),
                    Projection::Scalar => scalar(&runtime, root).map(Output::Scalar),
                    Projection::Elements => {
                        let values = children(&runtime, root)?;
                        values
                            .into_iter()
                            .map(|v| insert(&mut roots, &mut next, v))
                            .collect::<Answer<_>>()
                            .map(Output::Tokens)
                    }
                }
            }
        })();
        deliver_reply(&request.reply, result, &mut roots);
        if closing {
            break;
        }
    }
    // The arena drops before the VM, always on this thread, even on unwind.
}

fn deliver_reply(
    reply: &mpsc::SyncSender<Answer<Output>>,
    result: Answer<Output>,
    roots: &mut BTreeMap<u64, Root<'_>>,
) {
    // An abandoned synchronous caller must not retain newly returned VM roots.
    if let Err(mpsc::SendError(Ok(output))) = reply.send(result) {
        let ids = match output {
            Output::Token(id) => vec![id],
            Output::Tokens(ids) => ids,
            _ => Vec::new(),
        };
        for id in ids {
            roots.remove(&id);
        }
    }
}
fn insert<'a>(roots: &mut BTreeMap<u64, Root<'a>>, next: &mut u64, value: Root<'a>) -> Answer<u64> {
    let id = *next;
    *next = next
        .checked_add(1)
        .ok_or("Scheme handle identity exhausted")?;
    roots.insert(id, value);
    Ok(id)
}
fn encode<'a>(
    runtime: &'a GerbilRuntime,
    roots: &BTreeMap<u64, Root<'a>>,
    input: &Input,
) -> Result<Root<'a>, gerbil_scheme::NativeError> {
    match input {
        #[cfg(feature = "instance-backend")]
        Input::Shared(input) => encode(runtime, roots, input),
        Input::Void => runtime.void_object(),
        Input::Bool(v) => runtime.scalar_object(SchemeScalar::Boolean(*v)),
        Input::Integer(v) => {
            let (sign, mut bytes) = v.to_bytes_le();
            if sign == Sign::NoSign {
                bytes.clear();
            }
            runtime.big_integer(&BinaryInteger::new(sign == Sign::Minus, bytes)?)
        }
        Input::Real(v) => runtime.scalar_object(SchemeScalar::Flonum(*v)),
        Input::Text(v) => runtime
            .string_from_utf8(v)
            .map(gerbil_scheme::RootedSchemeString::into_object),
        Input::Bytes(v) => runtime
            .bytevector_from_bytes(v)
            .map(gerbil_scheme::RootedSchemeBytevector::into_object),
        Input::List(values) | Input::Vector(values) => {
            let values = values
                .iter()
                .map(|v| encode(runtime, roots, v))
                .collect::<Result<Vec<_>, _>>()?;
            runtime
                .collection_from_objects(
                    if matches!(input, Input::List(_)) {
                        CollectionKind::List
                    } else {
                        CollectionKind::Vector
                    },
                    &values.iter().collect::<Vec<_>>(),
                )
                .map(gerbil_scheme::RootedSchemeCollection::into_object)
        }
        Input::Table(values) => {
            let values = values
                .iter()
                .map(|(k, v)| Ok((encode(runtime, roots, k)?, encode(runtime, roots, v)?)))
                .collect::<Result<Vec<_>, gerbil_scheme::NativeError>>()?;
            runtime.hash_table(
                TableEquality::Structural,
                &values.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>(),
            )
        }
        Input::Reference(id) => {
            let root = roots.get(id).ok_or(gerbil_scheme::NativeError::Status {
                operation: "released Python handle",
                code: -1,
            })?;
            clone_root(runtime, root)
        }
    }
}
fn clone_root<'a>(
    runtime: &'a GerbilRuntime,
    root: &Root<'a>,
) -> Result<Root<'a>, gerbil_scheme::NativeError> {
    let vector = runtime.collection_from_objects(CollectionKind::Vector, &[root])?;
    let mut snapshot = RootedObjectBatch::default();
    vector.copy_into(&mut snapshot)?;
    snapshot.root_element(0)
}
fn scalar<'a>(runtime: &'a GerbilRuntime, root: &Root<'a>) -> Answer<Scalar> {
    let kind = root.as_ref().kind().map_err(|e| e.to_string())?;
    let result = (|| -> Result<Scalar, gerbil_scheme::NativeError> {
        match kind {
            ObjectKind::Void => Ok(Scalar::Null),
            ObjectKind::Boolean | ObjectKind::Flonum => match root.as_ref().scalar()? {
                SchemeScalar::Boolean(v) => Ok(Scalar::Boolean(v)),
                SchemeScalar::Flonum(v) => Ok(Scalar::Real(v)),
                _ => unreachable!(),
            },
            ObjectKind::ExactInteger => {
                let mut value = gerbil_scheme::BigIntegerBuffer::default();
                root.as_ref().copy_big_integer(&mut value)?;
                Ok(Scalar::Integer(BigInt::from_bytes_le(
                    if value.value().is_negative() {
                        Sign::Minus
                    } else {
                        Sign::Plus
                    },
                    value.value().magnitude(),
                )))
            }
            ObjectKind::String => Ok(Scalar::Text(
                clone_root(runtime, root)?
                    .into_string()?
                    .to_string()
                    .into_result()?,
            )),
            ObjectKind::Bytevector => {
                // Bytevector projection uses the upstream checked object conversion.
                let mut bytes = Vec::new();
                clone_root(runtime, root)?
                    .into_bytevector()?
                    .copy_to_vec(&mut bytes)
                    .into_result()?;
                Ok(Scalar::Bytes(bytes))
            }
            _ => Err(gerbil_scheme::NativeError::Status {
                operation: "no scalar projection; native value retained",
                code: -1,
            }),
        }
    })();
    result.map_err(|e| e.to_string())
}
fn children<'a>(runtime: &'a GerbilRuntime, root: &Root<'a>) -> Answer<Vec<Root<'a>>> {
    let mut snapshot = RootedObjectBatch::default();
    let kind = root.as_ref().kind().map_err(|e| e.to_string())?;
    (|| -> Result<_, gerbil_scheme::NativeError> {
        match kind {
            ObjectKind::GambitTable | ObjectKind::GerbilHashTable => {
                root.copy_table_entries(&mut snapshot)?;
            }
            ObjectKind::Pair | ObjectKind::Vector | ObjectKind::Nil => {
                let collection =
                    clone_root(runtime, root)?.into_collection(if kind == ObjectKind::Vector {
                        CollectionKind::Vector
                    } else {
                        CollectionKind::List
                    })?;
                collection.copy_into(&mut snapshot)?;
            }
            _ => {
                return Err(gerbil_scheme::NativeError::Status {
                    operation: "not a collection",
                    code: -1,
                });
            }
        }
        (0..snapshot.len())
            .map(|i| snapshot.root_element(i))
            .collect()
    })()
    .map_err(|e| e.to_string())
}
