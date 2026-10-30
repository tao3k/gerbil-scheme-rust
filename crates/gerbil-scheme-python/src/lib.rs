// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Shared packaging and Python embedding, independent of a library's DSL.
//!
//! The default feature is build-side/resource-only: Rust-only applications do
//! not acquire Python or a Scheme runtime dependency. Runtime modules are
//! loaded by the library's ordinary AOT graph, never by a Python compiler.

mod backend;
mod build_source;
#[cfg(all(feature = "owner-dispatch", target_os = "macos"))]
pub mod owner_dispatch;
#[cfg(all(feature = "owner-dispatch", target_os = "macos"))]
pub mod owner_flow;
pub use build_source::build_source_root;
mod owned_state;
pub use owned_state::{OwnedState, StateError};
mod owned_callable;
pub use owned_callable::OwnedCallable;
mod package;
pub use backend::{BackendError, select_backend};
pub use package::{PackageManifest, PackageResource, VerifiedPackage, sha256_file};
mod staging;
pub use staging::{ResourceTree, stage_resources, stage_source_inventory};
mod closure;
pub use closure::{
    ModuleBundleRequest, ModuleClosure, ModuleRoot, coalesce_module_resources, stage_module_closure,
};

#[cfg(any(feature = "scheme-backend", feature = "instance-backend"))]
mod runtime;
#[cfg(feature = "instance-backend")]
pub use runtime::{
    PythonImageCodeLease, PythonImageMappingReceipt, PythonImageMappingState,
    PythonInstanceProgram, PythonInstanceRequest, PythonPreparedRequest, open_instance_library,
};

#[cfg(any(feature = "scheme-backend", feature = "instance-backend"))]
pub use runtime::{
    PythonLibrary, PythonProgram, PythonRuntimeCapabilities, PythonSession, PythonValue,
    open_library,
};

#[cfg(feature = "python")]
pub use pyo3;
#[cfg(feature = "python")]
mod owned_value;
#[cfg(feature = "owned-values")]
mod values;
#[cfg(feature = "owned-values")]
pub use values::{OwnedValue, ValueLimits};
#[cfg(feature = "c-abi")]
pub mod c_abi;

#[cfg(test)]
asp_rust::asp_rust_cargo_test_gate!(
    config = {
        asp_rust::default_asp_rust_config()
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/backend.rs",
                [
                    asp_rust::RustOwnerResponsibility::PureDomainLogic,
                    asp_rust::RustOwnerResponsibility::PublicApi,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/package/inventory.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::Persistence,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/package/publication.rs",
                [
                    asp_rust::RustOwnerResponsibility::Persistence,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/closure.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/staging.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::Persistence,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/runtime.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                    asp_rust::RustOwnerResponsibility::AvailabilityCritical,
                    asp_rust::RustOwnerResponsibility::LatencySensitive,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/lib.rs",
                [asp_rust::RustOwnerResponsibility::PublicApi],
            ))
    }
);
