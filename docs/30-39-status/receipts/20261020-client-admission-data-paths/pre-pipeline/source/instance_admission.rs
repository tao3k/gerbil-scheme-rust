//! Verified package admission and fixed-owner startup.
use super::super::{Answer, Connection, PythonLibrary, Service};
use super::{GATE, Host, InstanceService, Registry, resources};
use crate::{PackageManifest, VerifiedPackage};
use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use pyo3::{PyResult, Python, exceptions::PyRuntimeError};
use std::{
    ffi::CStr,
    mem::ManuallyDrop,
    path::Path,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
/// Build-owned installed native image and its rooted exports, never Python code paths.
#[derive(Clone, Copy, Debug)]
pub struct PythonInstanceProgram {
    image: &'static str,
    workers: usize,
    dispatch: &'static CStr,
    configure: &'static CStr,
    apply: &'static CStr,
    finalizer: Option<&'static CStr>,
}
impl PythonInstanceProgram {
    /// Set bounded Rust dispatch workers; native VM owners remain fixed.
    #[must_use]
    pub const fn with_dispatch_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }
    /// # Safety
    /// The rooted export quiesces all Scheme services on its fixed owner.
    #[must_use]
    pub const unsafe fn with_finalizer(mut self, finalizer: &'static CStr) -> Self {
        self.finalizer = Some(finalizer);
        self
    }

    /// # Safety
    /// The packaged image obeys `IsolatedProgram`'s private runtime ABI; every
    /// export obeys its rooted unary ABI and uses this image's native runtime.
    #[must_use]
    pub const unsafe fn new(
        image: &'static str,
        dispatch: &'static CStr,
        configure: &'static CStr,
        apply: &'static CStr,
    ) -> Self {
        Self {
            image,
            dispatch,
            configure,
            apply,
            finalizer: None,
            workers: 2,
        }
    }
}
/// Open the packaged Scheme image; Rust provides cross-instance dispatch only.
/// # Errors
/// Preserves package validation, setup/drain failures and finite owner bounds.
pub fn open_instance_library(
    py: Python<'_>,
    program: PythonInstanceProgram,
    manifest: &PackageManifest,
    root: &Path,
    owners: usize,
) -> PyResult<PythonLibrary> {
    let service = py
        .detach(|| open(program, manifest, root, owners))
        .map_err(PyRuntimeError::new_err)?;
    Ok(PythonLibrary {
        service: Arc::new(service),
    })
}
fn open(
    program: PythonInstanceProgram,
    manifest: &PackageManifest,
    root: &Path,
    owners: usize,
) -> Answer<Service> {
    if !(1..=64).contains(&program.workers) {
        return Err("dispatch workers must be in 1..=64".into());
    }
    let package: VerifiedPackage = manifest.verify(root).map_err(|e| e.to_string())?;

    // SAFETY: maintainer program descriptor and verified package admit these exact exports.
    let mut image = unsafe {
        IsolatedProgram::from_image(root.join(program.image), program.dispatch)
            .map_err(|e| e.to_string())?
            .with_exports(&[program.configure, program.apply])
    };
    if let Some(finalizer) = program.finalizer {
        // SAFETY: PythonInstanceProgram admits this exact owner-local finalizer.
        image = unsafe { image.with_finalizer(finalizer) };
    }
    let mut pool = InstancePool::start(
        &image,
        InstanceConfig {
            owners,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
    )
    .map_err(|e| e.to_string())?;
    let dispatch = pool.dispatcher();
    let resource_root = pool.image_directory.clone();
    for owner in 0..owners {
        let paths = match resources::copy(
            &package,
            &resource_root.join(format!("owner-{owner}-modules")),
        ) {
            Ok(paths) => paths,
            Err(error) => {
                let receipt = pool.shutdown(GATE);
                if receipt.is_clean() {
                    let _ = resources::retire(&resource_root);
                }
                return Err(format!(
                    "Scheme module namespace: {error}; shutdown: {receipt:?}; resources: {}",
                    resource_root.display()
                ));
            }
        };
        let configure = program.configure;
        let result = dispatch
            .submit_native_on(owner, move |session| {
                let roots = paths
                    .iter()
                    .map(|path| session.string(path))
                    .collect::<Result<Vec<_>, _>>()?;
                let input = session.list(&roots.iter().collect::<Vec<_>>())?;
                session.call_export(configure, &input)?;
                let void = session.void_object()?;
                session.kind(&void)?;
                Ok(())
            })
            .and_then(|ticket| ticket.wait(GATE));
        if let Err(error) = result {
            let receipt = pool.shutdown(GATE);
            if receipt.is_clean() {
                let _ = resources::retire(&resource_root);
            }
            return Err(format!(
                "Scheme configure: {error}; shutdown: {receipt:?}; resources: {}",
                resource_root.display()
            ));
        }
    }
    let runtime = dispatch_runtime(program.workers, &mut pool, &resource_root)?;
    let states = pool.owners().iter().map(|owner| owner.state).collect();
    let instances = InstanceService {
        pid: std::process::id(),
        host: ManuallyDrop::new(Host {
            pool: Mutex::new(Some(pool)),
            runtime: Mutex::new(Some(Arc::new(runtime))),
            dispatch,
            registry: Arc::new(Registry::default()),
            closed: AtomicBool::new(false),
            receipt: Mutex::new(None),
            states,
            workers: program.workers,
            next_owner: std::sync::atomic::AtomicUsize::new(0),
            rayon: Mutex::new(None),
            apply: program.apply,
            resource_root,
            resources_removed: AtomicBool::new(false),
        }),
    };
    Ok(Service {
        connection: Mutex::<Option<Connection>>::new(None),
        pid: std::process::id(),
        package,
        namespace: None,
        instances: Some(instances),
    })
}

fn dispatch_runtime(
    workers: usize,
    pool: &mut InstancePool,
    resource_root: &Path,
) -> Answer<tokio::runtime::Runtime> {
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .thread_name("gerbil-dispatch")
        .enable_all()
        .build()
    {
        Ok(runtime) => Ok(runtime),
        Err(error) => {
            let receipt = pool.shutdown(GATE);
            if receipt.is_clean() {
                let _ = resources::retire(resource_root);
            }
            Err(format!("Tokio setup: {error}; shutdown: {receipt:?}"))
        }
    }
}
