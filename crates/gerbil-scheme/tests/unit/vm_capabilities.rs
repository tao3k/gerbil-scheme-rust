// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::VmCapabilities;

#[test]
fn linked_vm_capabilities_are_initialization_free() {
    let capabilities = VmCapabilities::linked();
    assert_eq!(capabilities, VmCapabilities::linked());
    assert!(capabilities.max_processors() >= 1);
    if !capabilities.thread_local_entry() {
        assert_eq!(capabilities.max_processors(), 1);
    }
    eprintln!(
        "VM-ABI multiple-vms={} thread-local-entry={} max-processors={} independent-worker-admission=unqualified",
        capabilities.multiple_vms(),
        capabilities.thread_local_entry(),
        capabilities.max_processors(),
    );
}

#[test]
fn vm_capability_probe_cannot_override_sdk_or_initialize_runtime() {
    let source = include_str!("../../../../ffi/runtime.c");
    let probe = source
        .split("uint32_t gerbil_scheme_rust_vm_capability_flags(void)")
        .nth(1)
        .expect("capability probe")
        .split("int64_t gerbil_scheme_rust_identity_i64")
        .next()
        .expect("probe end");
    assert!(probe.contains("#ifndef ___SINGLE_VM"));
    assert!(probe.contains("#ifndef ___SINGLE_THREADED_VMS"));
    assert!(probe.contains("return ___MAX_PROCESSORS;"));
    for forbidden in [
        "#define",
        "___setup(",
        "___setup_vmstate",
        "___SET_REAL_PSTATE",
        "___run(",
    ] {
        assert!(
            !probe.contains(forbidden),
            "probe must not contain {forbidden}"
        );
    }
}
