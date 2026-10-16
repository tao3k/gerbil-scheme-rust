// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Compiled VM ABI capabilities, independent of runtime initialization.

/// Capability bit for compiled storage supporting multiple VM instances.
pub const GERBIL_VM_CAP_MULTIPLE_VMS: u32 = 1;
/// Capability bit for thread-local processor-state selection at foreign entry.
pub const GERBIL_VM_CAP_THREAD_LOCAL_ENTRY: u32 = 2;

unsafe extern "C" {
    /// Return compiled VM capability bits without reading Scheme state.
    ///
    /// # Safety
    /// Requires the bridge and runtime to use the same SDK ABI.
    pub fn gerbil_scheme_rust_vm_capability_flags() -> u32;

    /// Return the compiled processor capacity, not the active processor count.
    ///
    /// # Safety
    /// Requires the bridge and runtime to use the same SDK ABI.
    pub fn gerbil_scheme_rust_vm_max_processors() -> u32;
}
