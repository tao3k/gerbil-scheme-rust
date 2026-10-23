//! Private native function table. No Scheme handle can leave this owner.
use super::{
    InstanceError, IsolatedProgram, OwnerReceipt, native_api::NativeApi, preflight::symbol,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

unsafe extern "C" {
    fn dlopen(path: *const c_char, flags: i32) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

pub(super) static INITIALIZATION_ATTEMPTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

static NEXT_IMAGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn next_identity() -> Result<u64, InstanceError> {
    use std::sync::atomic::Ordering;
    NEXT_IMAGE_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| InstanceError::InvalidConfig("image generation exhausted"))
}

type Unary = unsafe extern "C" fn(i64) -> i64;
#[derive(Debug)]
pub(super) struct Image {
    pub(super) identity: u64,
    pub(super) mapping: std::sync::Arc<super::mapping::Mapping>,
    cleanup: unsafe extern "C" fn() -> i32,
    retired: unsafe extern "C" fn() -> u64,
    roots: unsafe extern "C" fn() -> i64,
    gc: unsafe extern "C" fn() -> u64,
    pub(super) input: unsafe extern "C" fn(*const u8, u64) -> i64,
    pub(super) call: Unary,
    length: Unary,
    copy: unsafe extern "C" fn(i64, *mut u8, u64) -> i64,
    pub(super) release: unsafe extern "C" fn(i64) -> i32,
    pub(super) receipt: OwnerReceipt,
    pub(super) native: NativeApi,
    pub(super) exports: BTreeMap<CString, Unary>,
    finalizer: Option<Unary>,
    #[cfg(feature = "tokio-dispatch")]
    pub(super) service: Option<Unary>,
    pub(super) service_status: Cell<Option<i32>>,
    pub(super) retained: RefCell<BTreeMap<u64, i64>>,
    pub(super) next_retained: Cell<u64>,
    pub(super) service_root: Cell<Option<u64>>,
    live: bool,
    file: PathBuf,
    _owner: PhantomData<Rc<()>>,
}

// Reject an incompatible image before resolving its runtime functions.
unsafe fn validate_abi(handle: *mut c_void) -> Result<(), InstanceError> {
    // SAFETY: the admitted loaded image exposes a plain ABI-version query.
    let abi: unsafe extern "C" fn() -> u32 =
        unsafe { std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_abi")?) };
    if unsafe { abi() } == 4 {
        Ok(())
    } else {
        Err(InstanceError::Loader("instance ABI is not 4".into()))
    }
}

// Resolve maintainer exports before any native setup attempt.
unsafe fn program_exports(
    handle: *mut c_void,
    program: &IsolatedProgram,
) -> Result<ProgramExports, InstanceError> {
    let mut exports = BTreeMap::new();
    for name in &program.exports {
        // SAFETY: caller admits the loaded image and rooted unary signatures.
        exports.insert(name.clone(), unsafe {
            std::mem::transmute::<*mut c_void, Unary>(symbol(handle, name)?)
        });
    }
    let finalizer = program
        .finalizer
        .as_ref()
        .map(|name| {
            // SAFETY: same admitted image; no Scheme call or setup occurs here.
            unsafe {
                symbol(handle, name).map(|ptr| std::mem::transmute::<*mut c_void, Unary>(ptr))
            }
        })
        .transpose()?;
    #[cfg(feature = "tokio-dispatch")]
    let service = program
        .service
        .as_ref()
        .map(|name| {
            // SAFETY: this private rooted unary export shares the admitted image ABI.
            unsafe {
                symbol(handle, name)
                    .map(|address| std::mem::transmute::<*mut c_void, Unary>(address))
            }
        })
        .transpose()?;
    Ok(ProgramExports {
        exports,
        finalizer,
        #[cfg(feature = "tokio-dispatch")]
        service,
    })
}
struct ProgramExports {
    exports: BTreeMap<CString, Unary>,
    finalizer: Option<Unary>,
    #[cfg(feature = "tokio-dispatch")]
    service: Option<Unary>,
}

impl Image {
    pub(super) fn collection_count(&self) -> u64 {
        // SAFETY: borrowed active image, called only on its fixed native owner.
        unsafe { (self.gc)() }
    }
    pub(super) fn load(
        path: &Path,
        program: &IsolatedProgram,
        owner: usize,
        mapping: std::sync::Arc<super::mapping::Mapping>,
    ) -> Result<Self, InstanceError> {
        let start = Instant::now();
        let image_path = path.to_owned();
        let opened = open_image(path)?;
        mapping.loaded(opened);
        let handle = opened.as_ptr();
        let mut preflight =
            super::preflight::Preflight::new(std::sync::Arc::clone(&mapping), image_path.clone());
        // Resolve every required symbol before setup, so a missing symbol never
        // leaves an initialized VM without its cleanup function. Only images
        // that never attempted native setup may be unloaded on rejection.
        // SAFETY: signatures are the documented instance ABI and strong-root ABI.
        unsafe {
            validate_abi(handle)?;
            let setup: unsafe extern "C" fn(u32) -> i32 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_setup")?);
            let cleanup: unsafe extern "C" fn() -> i32 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_cleanup")?);
            let retired: unsafe extern "C" fn() -> u64 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_io_retired_bytes")?);
            let state: unsafe extern "C" fn() -> usize =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_state")?);
            let processors: unsafe extern "C" fn() -> u32 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_rust_current_vm_processors")?);
            let roots: unsafe extern "C" fn() -> i64 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_rust_root_count_raw")?);
            let gc: unsafe extern "C" fn() -> u64 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_gc_count")?);
            let input =
                std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*const u8, u64) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_bytes_to_bytevector_root_raw")?,
                );
            let call = std::mem::transmute::<*mut c_void, Unary>(symbol(handle, &program.entry)?);
            let declared = program_exports(handle, program)?;
            let length = std::mem::transmute::<*mut c_void, Unary>(symbol(
                handle,
                c"gerbil_scheme_rust_root_bytevector_length_raw",
            )?);
            let copy =
                std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64, *mut u8, u64) -> i64>(
                    symbol(handle, c"gerbil_scheme_rust_root_bytevector_copy_raw")?,
                );
            let release = std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64) -> i32>(
                symbol(handle, c"gerbil_scheme_rust_root_release_raw")?,
            );
            let phase: unsafe extern "C" fn() -> u32 =
                std::mem::transmute(symbol(handle, c"gerbil_scheme_instance_setup_phase")?);
            let native = NativeApi::load(handle)?;
            mapping.bind_modules(handle)?;
            let identity = mapping.identity;
            INITIALIZATION_ATTEMPTED.store(true, std::sync::atomic::Ordering::Release);
            let status = setup(u32::try_from(owner).expect("bounded owner"));
            let setup_phase = phase();
            if status == 0 || setup_phase != 0 {
                preflight.setup_attempted();
            }
            if status != 0 {
                let diagnostic = native.setup_failure();
                let rollback = (setup_phase == 2)
                    .then(|| rollback_initialized(owner, roots, gc, cleanup, &image_path, retired));
                crate::instance_backend::poison_images();
                return Err(InstanceError::Initialization {
                    owner,
                    status,
                    phase: setup_phase,
                    diagnostic,
                    cleanup: rollback,
                });
            }
            let image = Self {
                identity,
                mapping,
                cleanup,
                retired,
                native,
                exports: declared.exports,
                finalizer: declared.finalizer,
                #[cfg(feature = "tokio-dispatch")]
                service: declared.service,
                service_status: Cell::new(None),
                retained: RefCell::new(BTreeMap::new()),
                next_retained: Cell::new(1),
                service_root: Cell::new(None),
                roots,
                gc,
                input,
                call,
                length,
                copy,
                release,
                receipt: OwnerReceipt {
                    owner,
                    state: state(),
                    processors: processors(),
                    setup_time: start.elapsed(),
                },
                live: true,
                file: image_path,
                _owner: PhantomData,
            };
            image.validate()
        }
    }

    fn validate(self) -> Result<Self, InstanceError> {
        // SAFETY: native setup succeeded; all observations stay on the owner.
        if self.receipt.state != 0 && self.receipt.processors == 1 && unsafe { (self.roots)() } == 0
        {
            return Ok(self);
        }
        let cleanup = self.finish();
        crate::instance_backend::poison_images();
        Err(InstanceError::Initialization {
            owner: cleanup.owner,
            status: -1,
            phase: 2,
            diagnostic: None,
            cleanup: Some(cleanup),
        })
    }

    pub(super) fn invoke(
        &self,
        request: &[u8],
        maximum_reply: usize,
    ) -> Result<Vec<u8>, InstanceError> {
        let request_len = u64::try_from(request.len())
            .map_err(|_| InstanceError::InvalidConfig("request too large"))?;
        // SAFETY: the byte span stays borrowed until this owner-local call ends.
        let input = unsafe { (self.input)(request.as_ptr(), request_len) };
        if input <= 0 {
            return Err(InstanceError::Native {
                operation: "input root",
                code: input,
            });
        }
        // SAFETY: build-owned export borrows input and transfers a fresh root.
        let result = unsafe { (self.call)(input) };
        // SAFETY: release the input on this same live VM, including rejection.
        let input_released = unsafe { (self.release)(input) };
        let response = if result > 0 {
            let response = self.copy_reply(result, maximum_reply);
            // SAFETY: result is the fresh root transferred by the export.
            let released = unsafe { (self.release)(result) };
            if released == 1 {
                response
            } else {
                Err(InstanceError::Native {
                    operation: "release result",
                    code: i64::from(released),
                })
            }
        } else {
            Err(InstanceError::Native {
                operation: "Scheme export",
                code: result,
            })
        };
        if input_released != 1 || self.balanced().is_err() {
            crate::instance_backend::poison_images();
            return Err(InstanceError::Native {
                operation: "root balance",
                code: -1,
            });
        }
        response
    }

    pub(super) fn copy_reply(&self, result: i64, maximum: usize) -> Result<Vec<u8>, InstanceError> {
        // SAFETY: result stays strongly rooted through the length and copy calls.
        let length = unsafe { (self.length)(result) };
        let size = usize::try_from(length).map_err(|_| InstanceError::Native {
            operation: "result bytevector",
            code: length,
        })?;
        if size > maximum {
            return Err(InstanceError::InvalidConfig(
                "reply exceeds configured bound",
            ));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|error| InstanceError::Loader(error.to_string()))?;
        bytes.resize(size, 0);
        // SAFETY: exact-sized writable Rust allocation; no heap pointer escapes.
        if unsafe {
            (self.copy)(
                result,
                bytes.as_mut_ptr(),
                u64::try_from(size).expect("bounded reply"),
            )
        } != length
        {
            return Err(InstanceError::Native {
                operation: "copy result",
                code: -1,
            });
        }
        Ok(bytes)
    }

    pub(super) fn balanced(&self) -> Result<(), InstanceError> {
        // SAFETY: no native job result can retain a non-Send Scheme root.
        let roots = unsafe { (self.roots)() };
        if roots != i64::try_from(self.retained.borrow().len()).unwrap_or(-1) {
            crate::instance_backend::poison_images();
            return Err(InstanceError::Native {
                operation: "native job root balance",
                code: roots,
            });
        }
        if !crate::instance_backend::images_ready() {
            return Err(InstanceError::Closed);
        }
        Ok(())
    }

    pub(super) fn rollback_retained(&self, first: u64) {
        let roots = self.retained.borrow_mut().split_off(&first);
        if self.service_root.get().is_some_and(|id| id >= first) {
            self.service_root.set(None);
        }
        for (_, root) in roots {
            // SAFETY: each removed arena entry owns exactly one local native root.
            if unsafe { (self.release)(root) } != 1 {
                crate::instance_backend::poison_images();
            }
        }
    }
    fn release_retained(&self) {
        self.service_root.set(None);
        for (_, root) in std::mem::take(&mut *self.retained.borrow_mut()) {
            // SAFETY: store exclusively owns these roots on this fixed owner.
            if unsafe { (self.release)(root) } != 1 {
                crate::instance_backend::poison_images();
            }
        }
    }
    pub(super) fn finish(mut self) -> super::OwnerShutdown {
        let finalizer_status = self
            .service_status
            .get()
            .filter(|code| *code != 0)
            .or_else(|| {
                self.finalizer.map(|finalize| {
                    // SAFETY: queue is drained; the admitted finalizer uses this owner's ABI.
                    unsafe {
                        let input = (self.native.collection)(std::ptr::null(), 0, 0);
                        if input <= 0 {
                            return -71;
                        }
                        let output = finalize(input);
                        let released = (self.release)(input);
                        if output > 0 && (self.release)(output) == 1 && released == 1 {
                            0
                        } else {
                            -71
                        }
                    }
                })
            })
            .or(self.service_status.get());
        let quiesced = finalizer_status.is_none_or(|status| status == 0);
        if quiesced {
            self.release_retained();
        }
        self.live = false;
        let start = Instant::now();
        // SAFETY: owner-local observations while the SDK is still initialized.
        let (roots, collections) = unsafe { ((self.roots)(), (self.gc)()) };
        let native_cleanup_attempted = quiesced && roots == 0;
        let status = if native_cleanup_attempted {
            // SAFETY: accepted queue drained, application finalizer succeeded,
            // and no external roots remain. Unverified quiescence is quarantined.
            unsafe { (self.cleanup)() }
        } else {
            finalizer_status
                .filter(|status| *status != 0)
                .unwrap_or(-72)
        };
        if roots != 0 || status != 0 {
            crate::instance_backend::poison_images();
        }
        let cleanup_time = start.elapsed();
        let (image_file_removed, image_file_error) = super::retirement::file(&self.file, status);
        self.mapping
            .cleaned(native_cleanup_attempted && roots == 0 && status == 0);
        super::OwnerShutdown {
            owner: self.receipt.owner,
            roots_before_cleanup: roots,
            gc_collections: collections,
            native_cleanup_attempted,
            status,
            cleanup_time,
            image_file_removed,
            image_file_error,
            // SAFETY: plain adapter counter, independent of finalized VM memory.
            native_io_released_bytes: unsafe { (self.retired)() },
            finalizer_status,
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.live {
            // Unwinding has no successful application-finalizer receipt. Do not
            // enter Scheme, release roots or destroy a possibly active heap.
            // Keep the mapping, heap and file for process-terminal quarantine.
            self.live = false;
            self.mapping.quarantine();
            crate::instance_backend::poison_images();
        }
    }
}

fn rollback_initialized(
    owner: usize,
    roots: unsafe extern "C" fn() -> i64,
    gc: unsafe extern "C" fn() -> u64,
    cleanup: unsafe extern "C" fn() -> i32,
    path: &Path,
    retired: unsafe extern "C" fn() -> u64,
) -> super::OwnerShutdown {
    let start = Instant::now();
    // SAFETY: the adapter confirmed completed setup; all observations precede
    // cleanup on the initializing owner, and no dead VM state is accessed.
    let (roots_before_cleanup, gc_collections, status) = unsafe { (roots(), gc(), cleanup()) };
    let cleanup_time = start.elapsed();
    let (image_file_removed, image_file_error) = super::retirement::file(path, status);
    super::OwnerShutdown {
        owner,
        roots_before_cleanup,
        gc_collections,
        native_cleanup_attempted: true,
        status,
        cleanup_time,
        image_file_removed,
        image_file_error,
        // SAFETY: plain adapter counter, independent of finalized VM memory.
        native_io_released_bytes: unsafe { retired() },
        finalizer_status: None,
    }
}

fn loader_message() -> String {
    // SAFETY: read the thread-local loader diagnostic immediately after failure.
    let error = unsafe { dlerror() };
    if error.is_null() {
        "dlopen failed".into()
    } else {
        // SAFETY: loader-owned NUL-terminated error for this thread.
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

fn open_image(path: &Path) -> Result<std::ptr::NonNull<c_void>, InstanceError> {
    let encoded = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|e| InstanceError::Loader(e.to_string()))?;
    // SAFETY: Image::load's trusted descriptor admits this image; RTLD_LOCAL
    // and its export allow-list isolate the embedded Gambit symbols.
    let handle = unsafe { dlopen(encoded.as_ptr(), 2 | 4) };
    std::ptr::NonNull::new(handle).ok_or_else(|| {
        let message = loader_message();
        let _ = std::fs::remove_file(path);
        InstanceError::Loader(message)
    })
}
