// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! SDK ABI observations. These are prerequisites, never execution admission.

/// VM capabilities compiled into the linked bridge using the selected SDK.
///
/// This receipt does not prove independent-VM initialization, Gerbil module
/// isolation, SMP activation, or safe concurrent foreign calls. The current
/// `GerbilRuntime` remains a process-global, one-shot owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmCapabilities {
    flags: u32,
    max_processors: u32,
}

impl VmCapabilities {
    /// Inspect the linked ABI without initializing or entering a Scheme VM.
    #[must_use]
    pub fn linked() -> Self {
        // SAFETY: both helpers read compile-time constants only. The build
        // owner pairs bridge compilation and linking with the same SDK.
        unsafe {
            Self {
                flags: gerbil_scheme_sys::gerbil_scheme_rust_vm_capability_flags(),
                max_processors: gerbil_scheme_sys::gerbil_scheme_rust_vm_max_processors(),
            }
        }
    }

    /// Whether the ABI includes storage for multiple VM instances.
    #[must_use]
    pub fn multiple_vms(self) -> bool {
        self.flags & gerbil_scheme_sys::GERBIL_VM_CAP_MULTIPLE_VMS != 0
    }

    /// Whether foreign entry selects processor state through thread-local state.
    ///
    /// A false result rules out per-thread VM selection through the ordinary
    /// generated entry ABI. A true result still requires lifecycle qualification.
    #[must_use]
    pub fn thread_local_entry(self) -> bool {
        self.flags & gerbil_scheme_sys::GERBIL_VM_CAP_THREAD_LOCAL_ENTRY != 0
    }

    /// Compiled capacity per VM, not requested or active processor population.
    #[must_use]
    pub fn max_processors(self) -> u32 {
        self.max_processors
    }
}
