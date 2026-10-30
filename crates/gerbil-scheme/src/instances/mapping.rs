//! Explicit code ownership, independent of owner-local Scheme heap roots.
use std::{
    ffi::c_void,
    ptr::NonNull,
    sync::{Arc, Mutex},
};

/// Observed native-code ownership; retaining code does not retain a live VM.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageMappingState {
    /// An owner has not completed its actual native join.
    OwnerLive,
    /// The provider did not authorize unload of this initialized image.
    RetainedByPolicy,
    /// Host callbacks or foreign-thread TLS still hold explicit code leases.
    RetainedByLeases(usize),
    /// Cleanup or owner completion was not verified; never unload this image.
    Quarantined,
    /// A post-join dlclose is in progress; native destructors may still run.
    Unloading,
    /// This backend's dlopen reference has been balanced successfully.
    /// Other loader references, if any, can still retain the mapping.
    Released,
    /// The loader rejected release; preserve the handle and report the failure.
    ReleaseFailed(String),
}
/// Mapping receipt is separate from VM cleanup and file retirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageMappingReceipt {
    pub owner: usize,
    pub generation: u64,
    pub code_leases: usize,
    pub pending_module_closes: Option<u64>,
    pub state: ImageMappingState,
}
#[derive(Debug)]
struct Handle(NonNull<c_void>);
// SAFETY: Darwin loader handles may be closed from another thread. We never
// expose the pointer, and transfer it only after authorized, clean owner join.
unsafe impl Send for Handle {}
unsafe extern "C" {
    fn dlclose(handle: *mut c_void) -> i32;
}
#[derive(Debug, Default)]
enum Heap {
    #[default]
    Uninitialized,
    Live,
    Clean,
    Quarantined,
}
#[derive(Debug, Default)]
enum Loader {
    Resident(Handle),
    Releasing,
    #[default]
    Released,
    Failed {
        _handle: Handle,
        error: String,
    },
}
#[derive(Debug, Default)]
enum Modules {
    #[default]
    Held,
    Releasing,
    Drained,
    Failed(i32),
}
#[derive(Debug, Default)]
struct State {
    loader: Loader,
    heap: Heap,
    joined: bool,
    leases: usize,
    native_modules: Option<super::mapping_modules::NativeModules>,
    modules: Modules,
    pending_module_closes: Option<u64>,
}
#[derive(Debug)]
pub(super) struct Mapping {
    pub(super) owner: usize,
    pub(super) identity: u64,
    authorized: bool,
    state: Mutex<State>,
}
impl Mapping {
    pub(super) fn new(owner: usize, identity: u64, authorized: bool) -> Arc<Self> {
        Arc::new(Self {
            owner,
            identity,
            authorized,
            state: Mutex::new(State::default()),
        })
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn loaded(&self, handle: NonNull<c_void>) {
        self.lock().loader = Loader::Resident(Handle(handle));
    }
    pub(super) fn initialized(&self) {
        self.lock().heap = Heap::Live;
    }
    pub(super) unsafe fn bind_modules(
        &self,
        handle: *mut c_void,
    ) -> Result<(), super::InstanceError> {
        // SAFETY: preflight admitted this mapped ABI-v4 image.
        let api = unsafe { super::mapping_modules::NativeModules::load(handle)? };
        self.lock().native_modules = Some(api);
        Ok(())
    }
    pub(super) fn cleaned(&self, clean: bool) {
        let api = self.lock().native_modules;
        let pending = api.map(super::mapping_modules::NativeModules::pending);
        let mut state = self.lock();
        state.heap = if clean && api.is_some() {
            Heap::Clean
        } else {
            Heap::Quarantined
        };
        state.pending_module_closes = pending;
    }
    pub(super) fn quarantine(&self) {
        self.lock().heap = Heap::Quarantined;
    }
    fn take_resident(state: &mut State) -> Option<Handle> {
        if !matches!(state.loader, Loader::Resident(_)) {
            return None;
        }
        let Loader::Resident(handle) = std::mem::replace(&mut state.loader, Loader::Releasing)
        else {
            unreachable!("resident loader")
        };
        Some(handle)
    }
    /// Setup has never been attempted. Only loader-owned state can be retired;\n    /// no native SDK transaction or application finalizer runs on this mapping.
    pub(super) fn reject_uninitialized(&self) -> bool {
        let handle = Self::take_resident(&mut self.lock());
        handle.is_none_or(|handle| self.close_handle(handle))
    }
    fn close_handle(&self, handle: Handle) -> bool {
        // SAFETY: either preflight rejection or authorized clean native join
        // with zero leases. Never hold a Rust mutex while foreign hooks execute.
        let status = unsafe { dlclose(handle.0.as_ptr()) };
        let mut state = self.lock();
        if status == 0 {
            state.loader = Loader::Released;
            return true;
        }
        state.loader = Loader::Failed {
            _handle: handle,
            error: format!("dlclose failed with status {status}"),
        };
        crate::instance_backend::poison_images();
        false
    }
    pub(super) fn joined(&self, panicked: bool) {
        {
            let mut state = self.lock();
            state.joined = true;
            if panicked || matches!(state.heap, Heap::Live) {
                state.heap = Heap::Quarantined;
            }
        }
        self.try_release();
    }
    fn try_release(&self) {
        let api = {
            let mut state = self.lock();
            if !state.joined
                || !matches!(state.heap, Heap::Clean)
                || state.leases != 0
                || !matches!(state.loader, Loader::Resident(_))
                || !matches!(state.modules, Modules::Held)
            {
                return;
            }
            let Some(api) = state.native_modules else {
                state.heap = Heap::Quarantined;
                return;
            };
            state.modules = Modules::Releasing;
            api
        };
        // Native module destructors may release other host-owned code leases.
        // No Rust mutex is held; the base image remains resident throughout.
        let status = api.release();
        let pending = Some(api.pending());
        let handle = {
            let mut state = self.lock();
            state.pending_module_closes = pending;
            if status != 0 || pending != Some(0) {
                state.modules = Modules::Failed(status);
                crate::instance_backend::poison_images();
                return;
            }
            state.modules = Modules::Drained;
            if !self.authorized {
                return;
            }
            Self::take_resident(&mut state)
        };
        if let Some(handle) = handle {
            self.close_handle(handle);
        }
    }
    pub(super) fn snapshot(&self) -> ImageMappingReceipt {
        let state = self.lock();
        let status = match (&state.modules, &state.loader) {
            (Modules::Failed(code), _) => {
                ImageMappingState::ReleaseFailed(format!("SDK module dlclose failed: {code}"))
            }
            (Modules::Releasing, _) => ImageMappingState::Unloading,
            (_, Loader::Failed { error, .. }) => ImageMappingState::ReleaseFailed(error.clone()),
            _ if matches!(state.heap, Heap::Quarantined) => ImageMappingState::Quarantined,
            (_, Loader::Released) => ImageMappingState::Released,
            (_, Loader::Resident(_)) if !state.joined => ImageMappingState::OwnerLive,
            (_, Loader::Resident(_)) if !self.authorized => ImageMappingState::RetainedByPolicy,
            (_, Loader::Resident(_)) if state.leases != 0 => {
                ImageMappingState::RetainedByLeases(state.leases)
            }
            (_, Loader::Releasing | Loader::Resident(_)) => ImageMappingState::Unloading,
        };
        ImageMappingReceipt {
            owner: self.owner,
            generation: self.identity,
            code_leases: state.leases,
            pending_module_closes: state.pending_module_closes,
            state: status,
        }
    }
    pub(super) fn lease(self: &Arc<Self>) -> ImageCodeLease {
        let mut state = self.lock();
        assert!(matches!(state.loader, Loader::Resident(_)));
        state.leases = state
            .leases
            .checked_add(1)
            .expect("image code lease count exhausted");
        if let Some(api) = state.native_modules {
            api.set_leases(state.leases);
        }
        ImageCodeLease {
            mapping: Arc::clone(self),
        }
    }
}
/// Owned base/SDK-module code lifetime, transferable across Rust threads. This is not a
/// Scheme root and does not authorize entering a finalized VM. Hold it until
/// every callback is revoked or foreign-thread TLS destructor has returned.
/// Dropping the last lease may execute native unload hooks after owner join.
#[derive(Debug)]
pub struct ImageCodeLease {
    mapping: Arc<Mapping>,
}
impl Clone for ImageCodeLease {
    fn clone(&self) -> Self {
        self.mapping.lease()
    }
}
impl ImageCodeLease {
    /// Retire this code lease and observe the resulting mapping state.
    /// Native unload hooks may execute; this never re-enters the Scheme VM.
    #[must_use]
    pub fn release(self) -> ImageMappingReceipt {
        let mapping = Arc::clone(&self.mapping);
        drop(self);
        mapping.snapshot()
    }

    #[must_use]
    /// Observe mapping ownership without entering Scheme.
    pub fn mapping(&self) -> ImageMappingReceipt {
        self.mapping.snapshot()
    }
}
impl Drop for ImageCodeLease {
    fn drop(&mut self) {
        {
            let mut state = self.mapping.lock();
            state.leases -= 1;
            if let Some(api) = state.native_modules {
                api.set_leases(state.leases);
            }
        }
        self.mapping.try_release();
    }
}
