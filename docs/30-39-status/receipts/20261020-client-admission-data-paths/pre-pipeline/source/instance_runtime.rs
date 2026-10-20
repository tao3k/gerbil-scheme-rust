//! Tokio schedules owned requests; fixed native owners retain all Scheme objects.
use super::{Answer, Command, Output};
use gerbil_scheme::instances::{
    InstanceDispatcher, InstanceError, InstancePool, RetainedValue, ShutdownReceipt,
};
use std::{
    collections::BTreeMap,
    ffi::CStr,
    mem::ManuallyDrop,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
#[path = "instance_batch.rs"]
mod batch;
#[path = "instance_projection.rs"]
mod projection;
use super::resources;
#[path = "instance_values.rs"]
mod values;
use values::execute;
#[path = "instance_trace.rs"]
mod trace;
use trace::prepare;
#[path = "instance_admission.rs"]
mod admission;
pub use admission::{PythonInstanceProgram, open_instance_library};
const GATE: Duration = Duration::from_secs(5);
#[derive(Debug, Default)]
struct Registry {
    trace: trace::Trace,
    next: AtomicU64,
    values: Mutex<BTreeMap<u64, RetainedValue>>,
}
impl Registry {
    fn get(&self, id: u64) -> Result<RetainedValue, InstanceError> {
        self.values
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id)
            .copied()
            .ok_or(InstanceError::Closed)
    }
    fn insert(&self, value: RetainedValue) -> Answer<u64> {
        let id = self
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "Python token exhausted")?
            + 1;
        self.values
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id, value);
        Ok(id)
    }
}
enum NativeOutput {
    Plain(Output),
    Root(RetainedValue),
    Children(Vec<RetainedValue>),
}
#[derive(Debug)]
struct Host {
    pool: Mutex<Option<InstancePool>>,
    runtime: Mutex<Option<Arc<tokio::runtime::Runtime>>>,
    dispatch: InstanceDispatcher,
    registry: Arc<Registry>,
    closed: AtomicBool,
    receipt: Mutex<Option<ShutdownReceipt>>,
    states: Vec<usize>,
    workers: usize,
    next_owner: AtomicUsize,
    rayon: Mutex<Option<Arc<rayon::ThreadPool>>>,
    apply: &'static CStr,
    resource_root: std::path::PathBuf,
    resources_removed: AtomicBool,
}
/// Runtime resources are deliberately not destructed in an inherited fork child.
#[derive(Debug)]
pub(super) struct InstanceService {
    host: ManuallyDrop<Host>,
    pid: u32,
}
impl Drop for InstanceService {
    fn drop(&mut self) {
        if self.pid == std::process::id() {
            let _ = self.close();
            // SAFETY: this process owns Host; no automatic ManuallyDrop destructor runs.
            unsafe { ManuallyDrop::drop(&mut self.host) };
        }
    }
}
impl InstanceService {
    pub(in super::super) fn host_span(&self, kind: &'static str, started: std::time::Instant) {
        self.host.registry.trace.host(kind, started);
    }
    pub(in super::super) fn set_owner_profiling(&self, enabled: bool) -> Answer<()> {
        let _runtime = self.runtime()?;
        self.host.registry.trace.enable(enabled);
        Ok(())
    }
    pub(in super::super) fn owner_profile(&self) -> Answer<String> {
        let _runtime = self.runtime()?;
        self.host.registry.trace.take()
    }
    pub(super) fn resource_directory(&self) -> String {
        self.host.resource_root.display().to_string()
    }
    pub(super) fn collect(&self) -> Answer<Vec<[f64; 4]>> {
        let _runtime = self.runtime()?;
        let mut pending = Vec::new();
        for owner in 0..self.owners() {
            pending.push(
                self.host
                    .dispatch
                    .submit_native_on(owner, |session| {
                        session.collect()?;
                        session.usage()
                    })
                    .map_err(|e| e.to_string())?,
            );
        }
        pending
            .into_iter()
            .map(|ticket| ticket.wait(GATE).map_err(|e| e.to_string()))
            .collect()
    }
    pub(super) fn owners(&self) -> usize {
        self.host.states.len()
    }
    pub(super) fn resources_removed(&self) -> bool {
        self.host.resources_removed.load(Ordering::Acquire)
    }
    pub(super) fn states(&self) -> Vec<usize> {
        self.host.states.clone()
    }
    pub(super) fn shutdown_receipt(&self) -> Option<ShutdownReceipt> {
        self.host
            .receipt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    fn runtime(&self) -> Answer<Arc<tokio::runtime::Runtime>> {
        if self.pid != std::process::id() {
            return Err("inherited Scheme host after fork".into());
        }
        if self.host.closed.load(Ordering::Acquire) {
            return Err("Scheme library is closed".into());
        }
        self.host
            .runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .ok_or_else(|| "Scheme host is closed".into())
    }
    pub(in super::super) fn token_owner(&self, id: u64) -> Answer<usize> {
        self.host
            .registry
            .get(id)
            .map(RetainedValue::owner)
            .map_err(|e| e.to_string())
    }
    fn validate_input_owner(&self, input: &super::Input, owner: usize) -> Answer<()> {
        match input {
            super::Input::Reference(id) => {
                if self.token_owner(*id)? != owner {
                    return Err("Scheme reference belongs to another owner".into());
                }
                Ok(())
            }
            super::Input::List(values) | super::Input::Vector(values) => values
                .iter()
                .try_for_each(|value| self.validate_input_owner(value, owner)),
            super::Input::Table(values) => values.iter().try_for_each(|(key, value)| {
                self.validate_input_owner(key, owner)?;
                self.validate_input_owner(value, owner)
            }),
            _ => Ok(()),
        }
    }
    fn reserve_owners(&self, count: usize) -> usize {
        let owners = self.owners();
        self.host
            .next_owner
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                Some((next + count % owners) % owners)
            })
            .unwrap_or_default()
    }
    pub(in super::super) fn call_stateless(&self, input: super::Input) -> Answer<Output> {
        if input.contains_reference() {
            return Err("stateless requests cannot contain owner-bound Scheme references".into());
        }
        self.request_on(self.reserve_owners(1), Command::Call(input))
    }
    fn owner(&self, command: &Command) -> Answer<usize> {
        let id = match command {
            Command::Inspect(id, _)
            | Command::Apply(id, _)
            | Command::Release(id)
            | Command::Identical(id, _) => Some(*id),
            _ => None,
        };
        id.map_or(Ok(0), |id| {
            self.host
                .registry
                .get(id)
                .map(RetainedValue::owner)
                .map_err(|e| e.to_string())
        })
    }
    fn publish(&self, value: NativeOutput) -> Answer<Output> {
        match value {
            NativeOutput::Plain(value) => Ok(value),
            NativeOutput::Root(value) => self.host.registry.insert(value).map(Output::Token),
            NativeOutput::Children(values) => values
                .into_iter()
                .map(|value| self.host.registry.insert(value))
                .collect::<Answer<_>>()
                .map(Output::Tokens),
        }
    }
    pub(super) fn request(&self, command: Command) -> Answer<Output> {
        if matches!(command, Command::Release(_)) && self.host.closed.load(Ordering::Acquire) {
            return Ok(Output::Unit);
        }
        let owner = self.owner(&command)?;
        self.request_on(owner, command)
    }
    pub(super) fn request_on(&self, owner: usize, command: Command) -> Answer<Output> {
        if owner >= self.owners() {
            return Err("native owner index out of range".into());
        }
        match &command {
            Command::Encode(input) | Command::Call(input) | Command::Apply(_, input) => {
                self.validate_input_owner(input, owner)?;
            }
            _ => (),
        }
        let runtime = self.runtime()?;
        let dispatch = self.host.dispatch.clone();
        let registry = Arc::clone(&self.host.registry);
        let apply = self.host.apply;
        let job = prepare(command, registry, apply, owner);
        let task = runtime.spawn(async move {
            dispatch
                .submit_native_async_on(owner, job)?
                .wait_completed(GATE)
                .await
                .map(|(value, _)| value)
        });
        let value = runtime
            .block_on(task)
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        self.publish(value)
    }
    pub(super) fn close(&self) -> Answer<()> {
        if self.pid != std::process::id() {
            return Err("inherited Scheme host after fork".into());
        }
        self.host.closed.store(true, Ordering::Release);
        let pool = self
            .host
            .pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(mut pool) = pool {
            let receipt = pool.shutdown(GATE);
            let clean = receipt.is_clean();
            *self
                .host
                .receipt
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(receipt.clone());
            if !clean {
                *self
                    .host
                    .pool
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(pool);
                return Err(format!("Scheme shutdown incomplete: {receipt:?}"));
            }
            self.host
                .registry
                .values
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clear();
        }
        let native_clean = self
            .host
            .receipt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(ShutdownReceipt::is_clean);
        if native_clean {
            resources::retire(&self.host.resource_root)
                .map_err(|error| format!("runtime namespace retirement: {error}"))?;
            self.host.resources_removed.store(true, Ordering::Release);
        }
        self.host
            .rayon
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        self.host
            .runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        Ok(())
    }
}
