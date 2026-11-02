//! Direct C ABI package operations on the existing native runtime owner.
use super::{GerbilRuntime, NativeError};
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
    /// unrelated host environment mutation or subprocess work must not overlap.
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

    /// Stage the compiler-owned complete program manifest for a package entry.
    /// # Errors
    /// Returns owner/image/transport errors; nonzero status retains staging failure.
    pub fn stage_package_program(
        &self,
        directory: &str,
        source: &str,
        output: &str,
    ) -> Result<i32, NativeError> {
        self.check_thread()?;
        let mut code = 0;
        // SAFETY: all UTF-8 borrows and output remain live through foreign return.
        let status = unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_package_stage(
                directory.into(),
                source.into(),
                output.into(),
                &raw mut code,
            )
        };
        package_result("package program stage", status, code)
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
