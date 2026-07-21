// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{
    GERBIL_SCHEME_RUST_ABI_VERSION, GerbilRuntime, GerbilStatus, GerbilValueProvenance, NativeError,
};

#[test]
fn calls_scalar_export_in_process() {
    let runtime = GerbilRuntime::initialize().expect("initialize in-process Gerbil runtime");
    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::AlreadyInitialized)
    ));
    assert_eq!(
        runtime.abi_version().unwrap(),
        GERBIL_SCHEME_RUST_ABI_VERSION
    );
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    let value = runtime.runtime_sentinel_value().unwrap();
    assert_eq!(value.provenance(), GerbilValueProvenance::RuntimeSentinel);
    assert_eq!(value.is_pair().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_list().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_null().status(), Some(GerbilStatus::InvalidValue));
    let scheme_null = runtime
        .fixture_null_value()
        .expect("export Scheme null object through native runtime");
    assert_ne!(scheme_null.as_raw(), 0);
    assert_eq!(
        scheme_null.provenance(),
        GerbilValueProvenance::SchemeObjectExport
    );
    assert_eq!(
        scheme_null.is_pair().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        scheme_null.is_list().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        scheme_null.is_null().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        runtime.add_i64(i64::MAX, 1),
        Err(NativeError::IntegerOverflow {
            left: i64::MAX,
            right: 1,
        })
    );
    drop(runtime);

    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::RuntimeFinalized)
    ));
}
