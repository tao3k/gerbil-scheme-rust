// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Static program descriptors; runtime policy stays with the downstream owner.

use super::types::RootedSchemeOwner;
use super::{GerbilRuntime, NativeError, NativeResult, RootedSchemeString};

/// A build-owned AOT module graph containing the native bridge.
#[derive(Clone, Copy, Debug)]
pub struct LinkedGerbilProgram {
    pub(super) linker: gerbil_scheme_sys::GerbilProgramLinker,
}

impl LinkedGerbilProgram {
    /// Bind a generated Gambit linker to its safe lifecycle owner.
    ///
    /// # Safety
    ///
    /// `linker` must belong to the linked Gambit version and include exactly
    /// one native bridge module with ABI v1. All graph initialization must
    /// return normally. The function and graph must live for the process.
    #[must_use]
    pub const unsafe fn from_linker(linker: gerbil_scheme_sys::GerbilProgramLinker) -> Self {
        Self { linker }
    }
}

/// Statically linked export transferring a fresh root for one Scheme string.
#[derive(Clone, Copy, Debug)]
pub struct LinkedStringExport<'runtime> {
    runtime: &'runtime GerbilRuntime,
    export: unsafe extern "C" fn() -> i64,
}

impl GerbilRuntime {
    /// Register a build-owned Scheme export against this live runtime.
    ///
    /// # Safety
    ///
    /// The export must already be initialized by this runtime's AOT graph, catch Scheme
    /// exceptions, return zero on failure, and otherwise transfer one unique
    /// live string root from `gerbil-rs-root-string`. It may not retain Rust
    /// pointers, reenter this runtime owner, or perform runtime cleanup.
    pub unsafe fn bind_string_export(
        &self,
        export: unsafe extern "C" fn() -> i64,
    ) -> Result<LinkedStringExport<'_>, NativeError> {
        self.check_thread()?;
        Ok(LinkedStringExport {
            runtime: self,
            export,
        })
    }
}

impl<'runtime> LinkedStringExport<'runtime> {
    /// Call a registered string projection, retaining its GC root until drop.
    pub fn call(&self) -> NativeResult<RootedSchemeString<'runtime>> {
        let result = (|| {
            self.runtime.check_thread()?;
            // SAFETY: descriptor construction guarantees an initialized export
            // and transfers unique root ownership on this runtime thread.
            let root = gerbil_scheme_sys::GerbilRootId(unsafe { (self.export)() });
            let owner = RootedSchemeOwner::new(
                gerbil_scheme_sys::GerbilStatus::Ok,
                root,
                "linked string export",
            )?;
            let value = RootedSchemeString { owner };
            // Type-check the live root before it can escape. On error the owner
            // releases it; zero roots never become owners.
            value.len().into_result()?;
            Ok(value)
        })();
        result.into()
    }
}
