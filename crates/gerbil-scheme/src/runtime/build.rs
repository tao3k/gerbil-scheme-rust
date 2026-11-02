//! Direct C ABI package operations on the existing native runtime owner.
use super::{CollectionKind, GerbilRuntime, IdentifierKind, NativeError, RootedSchemeObject};
use gerbil_scheme_sys::{GerbilBorrowedUtf8, GerbilStatus};
use std::sync::atomic::{AtomicBool, Ordering};

/// Cancellation never discards the result of an entered Scheme operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PackageCallOutcome {
    /// The entry fence rejected the operation; no Scheme call occurred.
    CancelledBeforeEntry,
    /// The foreign call returned, including its nonzero package failure code.
    Completed {
        /// Official package builder or `GxTest` result.
        code: i32,
        /// Cancellation was observed after entry; this is not artifact rollback.
        cancelled: bool,
    },
}

impl GerbilRuntime {
    /// Invoke the package's official `build.ss clean` action on this VM owner.
    /// Package code owns generated-output cleanup; this does not unload modules
    /// or classify artifacts in Rust. A new source cut requires a new host.
    /// Bind package paths explicitly before requesting cleanup.
    /// # Errors
    /// Returns checked owner/image/transport errors and preserves package status.
    pub fn clean_package(&self, directory: &str) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut code = 0;
        // SAFETY: owner checked; borrowed directory and output stay live.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_clean(directory.into(), &raw mut code)
        };
        package_result("package clean", status, code)
    }

    /// Apply the existing entry/drain cancellation contract to package cleanup.
    /// # Errors
    /// Preserves the same owner, image and transport errors as `clean_package`.
    pub fn clean_package_cancellable(
        &self,
        directory: &str,
        cancelled: &AtomicBool,
    ) -> Result<PackageCallOutcome, NativeError> {
        self.package_call(cancelled, || self.clean_package(directory))
    }

    /// Run a package build with a monotonic caller-owned cancellation flag.
    ///
    /// The flag must never be reset while this call is pending. The acquire
    /// load is the entry fence. A later cancellation waits for actual return.
    /// # Errors
    /// Retains owner, image and transport errors from the ordinary build call.
    pub fn build_package_cancellable(
        &self,
        directory: &str,
        cancelled: &AtomicBool,
    ) -> Result<PackageCallOutcome, NativeError> {
        self.package_call(cancelled, || self.build_package(directory))
    }

    /// Run official `GxTest` with the same entry/drain cancellation contract.
    /// # Errors
    /// Retains owner, image and transport errors from the ordinary test call.
    pub fn test_package_cancellable(
        &self,
        directory: &str,
        tests: &str,
        cancelled: &AtomicBool,
    ) -> Result<PackageCallOutcome, NativeError> {
        self.package_call(cancelled, || self.test_package(directory, tests))
    }

    fn package_call(
        &self,
        cancelled: &AtomicBool,
        call: impl FnOnce() -> Result<i32, NativeError>,
    ) -> Result<PackageCallOutcome, NativeError> {
        self.check_thread()?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(PackageCallOutcome::CancelledBeforeEntry);
        }
        let code = call()?;
        Ok(PackageCallOutcome::Completed {
            code,
            cancelled: cancelled.load(Ordering::Acquire),
        })
    }
    /// Bind this build host to one package prefix and its standard Gerbil load path.
    /// Calls scope and restore Gerbil's environment and loader paths. A different
    /// prefix on the same owner is refused. This is trusted build-host tooling;
    /// unrelated host environment access or subprocess work must not overlap.
    /// # Errors
    /// Returns owner/transport errors; nonzero Scheme status refuses rebinding.
    pub fn configure_package_paths(
        &self,
        prefix: &str,
        libraries: &str,
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut code = 0;
        // SAFETY: checked owner, call-scoped inputs and writable result.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_paths(
                prefix.into(),
                libraries.into(),
                &raw mut code,
            )
        };
        package_result("package paths", status, code)
    }

    /// Invoke the package's actual `build.ss` main with ordinary arguments.
    /// # Errors
    /// Returns checked owner/image/transport errors. Package failures retain their code.
    pub fn run_package_main(
        &self,
        directory: &str,
        arguments: &[&str],
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let arguments: Vec<GerbilBorrowedUtf8> =
            arguments.iter().map(|value| (*value).into()).collect();
        let mut code = 0;
        // SAFETY: the complete argument array, nested borrows and result remain live.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_main(
                directory.into(),
                arguments.as_ptr(),
                arguments.len(),
                &raw mut code,
            )
        };
        package_result("package main", status, code)
    }

    /// Execute the ordinary native `build.ss` entry without a Gerbil CLI child.
    /// # Errors
    /// Rejects thread/lifecycle errors and invalid UTF-8 transport. A nonzero
    /// returned code is a package build failure, not successful admission.
    /// Native tool execution temporarily owns child-process handling; do not
    /// run unrelated host subprocesses concurrently with this operation.
    pub fn build_package(&self, directory: &str) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut result = 0;
        // SAFETY: self proves the live owner; borrowed strings and output stay live.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_build(
                GerbilBorrowedUtf8::from(directory),
                &raw mut result,
            )
        };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "package build",
                code: status as i32,
            });
        }
        Ok(result)
    }

    /// Execute native `GxTest` on a package-relative file or test directory.
    /// # Errors
    /// Same owner/transport requirements as `build_package`. Native failure is
    /// retained as a nonzero result (`GxTest` returns 42 for failed checks).
    pub fn test_package(&self, directory: &str, tests: &str) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut result = 0;
        // SAFETY: self proves the live owner; all borrowed spans stay live.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_test(
                GerbilBorrowedUtf8::from(directory),
                GerbilBorrowedUtf8::from(tests),
                &raw mut result,
            )
        };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "package test",
                code: status as i32,
            });
        }
        Ok(result)
    }
    /// Compile an existing native build.ss specification, without parsing it in Rust.
    /// # Errors
    /// Same owner/transport requirements and native result semantics as `build_package`.
    pub fn build_package_spec(
        &self,
        directory: &str,
        specification: &str,
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut result = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_build_spec(
                GerbilBorrowedUtf8::from(directory),
                GerbilBorrowedUtf8::from(specification),
                &raw mut result,
            )
        };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "native package specification",
                code: status as i32,
            });
        }
        Ok(result)
    }

    /// Stage an already compiled entry through the SDK's native graph producer.
    /// Imports use the existing native symbol/list/root ABI, not a new carrier.
    /// # Errors
    /// Returns native owner, conversion or transport errors and native result codes.
    pub fn stage_package_program(
        &self,
        source: &str,
        output: &str,
        runtime_imports: &[&str],
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let imports = self.root_package_imports(runtime_imports)?;
        let mut result = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_stage(
                GerbilBorrowedUtf8::from(source),
                GerbilBorrowedUtf8::from(output),
                imports.root_id(),
                &raw mut result,
            )
        };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "native programme graph",
                code: status as i32,
            });
        }
        Ok(result)
    }

    /// Stage the SDK's existing native module resource closure.
    /// # Errors
    /// Returns owner/transport errors and nonzero native operation results.
    pub fn stage_package_load_closure(
        &self,
        runtime_imports: &[&str],
        output: &str,
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let imports = self.root_package_imports(runtime_imports)?;
        let mut result = 0;
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_load_closure(
                imports.root_id(),
                GerbilBorrowedUtf8::from(output),
                &raw mut result,
            )
        };
        if status != GerbilStatus::Ok {
            return Err(NativeError::Status {
                operation: "native load closure",
                code: status as i32,
            });
        }
        Ok(result)
    }

    fn root_package_imports(
        &self,
        runtime_imports: &[&str],
    ) -> Result<RootedSchemeObject<'_>, NativeError> {
        let symbols = runtime_imports
            .iter()
            .map(|name| self.intern_identifier(IdentifierKind::Symbol, name))
            .collect::<Result<Vec<_>, _>>()?;
        let references: Vec<_> = symbols.iter().collect();
        let imports = self
            .collection_from_objects(CollectionKind::List, &references)?
            .into_object();
        Ok(imports)
    }
}

fn package_result(
    operation: &'static str,
    status: GerbilStatus,
    code: i32,
) -> Result<i32, NativeError> {
    match status {
        GerbilStatus::Ok => Ok(code),
        _ => Err(NativeError::Status {
            operation,
            code: status as i32,
        }),
    }
}

/// Tooling is an exclusive build-host operation. Apply the existing compiler
/// projection before VM setup and restore it only after native teardown.
pub(super) struct CompilerEnvironment {
    previous: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)>,
}

impl std::fmt::Debug for CompilerEnvironment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompilerEnvironment")
            .finish_non_exhaustive()
    }
}

impl CompilerEnvironment {
    pub(super) fn enter() -> Self {
        // This command is an environment projection only; it is never spawned.
        let mut projection = std::process::Command::new("gsc");
        crate::tool_environment::configure_gerbil_compiler_environment(&mut projection);
        let previous = projection
            .get_envs()
            .map(|(name, _)| (name.to_owned(), std::env::var_os(name)))
            .collect();
        for (name, value) in projection.get_envs() {
            // SAFETY: default build tooling requires exclusive host environment access
            // from initialization through teardown. No Scheme owner exists yet.
            unsafe {
                restore_variable(name, value);
            }
        }
        Self { previous }
    }
}

impl Drop for CompilerEnvironment {
    fn drop(&mut self) {
        for (name, value) in &self.previous {
            // SAFETY: the same exclusive tooling lifetime covers this restore;
            // Gambit teardown has already installed surviving environment storage.
            unsafe {
                restore_variable(name, value.as_deref());
            }
        }
    }
}

unsafe fn restore_variable(name: &std::ffi::OsStr, value: Option<&std::ffi::OsStr>) {
    // SAFETY: callers retain the exclusive build-host environment contract.
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
