// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Source-policy gate; installed-wheel behavior is qualified separately.
asp_rust::asp_rust_cargo_test_gate!(
    config = {
        asp_rust::default_asp_rust_config()
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/api.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/lib.rs",
                [asp_rust::RustOwnerResponsibility::PublicApi],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/rust_backend.rs",
                [
                    asp_rust::RustOwnerResponsibility::PublicApi,
                    asp_rust::RustOwnerResponsibility::AvailabilityCritical,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "build.rs",
                [
                    asp_rust::RustOwnerResponsibility::ExternalDependency,
                    asp_rust::RustOwnerResponsibility::Persistence,
                    asp_rust::RustOwnerResponsibility::SecurityBoundary,
                ],
            ))
    }
);
