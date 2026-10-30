// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Python owns opaque tokens; all native roots stay on the original VM thread.
use crate::{OwnedValue as Scalar, PackageManifest, VerifiedPackage};
use gerbil_scheme::{
    BinaryInteger, CollectionKind, GerbilRuntime, LinkedGerbilProgram, ObjectKind,
    RootedObjectBatch, RootedSchemeObject, SchemeScalar, TableEquality, VmCapabilities,
};
use num_bigint::{BigInt, Sign};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
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
#[path = "instance_mapping.rs"]
mod mapping;
#[cfg(feature = "instance-backend")]
pub use instances::{PythonInstanceProgram, PythonInstanceRequest, open_instance_library};
#[cfg(feature = "instance-backend")]
pub use mapping::{PythonImageCodeLease, PythonImageMappingReceipt, PythonImageMappingState};
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
#[path = "runtime_input.rs"]
mod input;
#[path = "runtime_library.rs"]
mod library_api;
#[path = "runtime_value.rs"]
mod value_api;
use input::read;
#[path = "runtime_linked.rs"]
mod linked;
pub use linked::open_library;
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
#[derive(Clone, Copy, Debug)]
enum ExecutionKind {
    Linked,
    #[cfg(feature = "instance-backend")]
    Independent(usize),
}
#[derive(Debug)]
struct Service {
    kind: ExecutionKind,
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

/// One Python-visible library lifetime on its selected fixed Scheme owners.
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

fn token(service: &Arc<Service>, output: &Output) -> PyResult<PythonValue> {
    match output {
        Output::Token(v) => Ok(PythonValue::new(service, *v)),
        _ => Err(protocol()),
    }
}
fn protocol() -> PyErr {
    PyRuntimeError::new_err("Scheme bridge response mismatch")
}
