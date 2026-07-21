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
    exports_scheme_objects_and_traverses_pairs(&runtime);
    reports_overflow_and_finalized_runtime_boundaries(runtime);
}

fn exports_scheme_objects_and_traverses_pairs(runtime: &GerbilRuntime) {
    exports_runtime_sentinel(runtime);
    exports_null_object(runtime);
    exports_boolean_objects(runtime);
    exports_fixnum_object(runtime);
    exports_pair_object(runtime);
    exports_proper_list_object(runtime);
    exports_improper_list_object(runtime);
}

fn exports_runtime_sentinel(runtime: &GerbilRuntime) {
    let value = runtime.runtime_sentinel_value().unwrap();
    assert_eq!(value.provenance(), GerbilValueProvenance::RuntimeSentinel);
    assert_fail_closed_value(value);
}

fn exports_null_object(runtime: &GerbilRuntime) {
    let scheme_null = runtime
        .fixture_null_value()
        .expect("export Scheme null object through native runtime");
    assert_ne!(scheme_null.as_raw(), 0);
    assert_scheme_object_export(scheme_null);
    assert_eq!(scheme_null.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_null.is_list().as_result(), Ok(&true));
    assert_eq!(scheme_null.is_null().as_result(), Ok(&true));
    assert_eq!(scheme_null.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_null.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_fail_closed_traversal(scheme_null);
}

fn exports_boolean_objects(runtime: &GerbilRuntime) {
    let scheme_true = runtime
        .fixture_true_value()
        .expect("export Scheme true object through native runtime");
    assert_scheme_object_export(scheme_true);
    assert_eq!(scheme_true.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_true.is_boolean().as_result(), Ok(&true));
    assert_eq!(scheme_true.as_boolean().as_result(), Ok(&true));
    assert_fail_closed_traversal(scheme_true);
    assert_untrusted_raw_fail_closed(scheme_true);

    let scheme_false = runtime
        .fixture_false_value()
        .expect("export Scheme false object through native runtime");
    assert_scheme_object_export(scheme_false);
    assert_eq!(scheme_false.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_false.is_boolean().as_result(), Ok(&true));
    assert_eq!(scheme_false.as_boolean().as_result(), Ok(&false));
    assert_fail_closed_traversal(scheme_false);
    assert_untrusted_raw_fail_closed(scheme_false);
}

fn exports_fixnum_object(runtime: &GerbilRuntime) {
    let scheme_fixnum = runtime
        .fixture_fixnum_value()
        .expect("export Scheme fixnum object through native runtime");
    assert_scheme_object_export(scheme_fixnum);
    assert_eq!(scheme_fixnum.is_pair().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_list().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_null().as_result(), Ok(&false));
    assert_eq!(scheme_fixnum.is_boolean().as_result(), Ok(&false));
    assert_eq!(
        scheme_fixnum.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(scheme_fixnum.is_fixnum().as_result(), Ok(&true));
    assert_eq!(scheme_fixnum.as_fixnum().as_result(), Ok(&42));
    assert_eq!(scheme_fixnum.as_fixnum_i64().as_result(), Ok(&42));
    assert_fail_closed_traversal(scheme_fixnum);
    assert_untrusted_raw_fail_closed(scheme_fixnum);
}

fn exports_pair_object(runtime: &GerbilRuntime) {
    let pair = runtime
        .fixture_pair_value()
        .expect("export Scheme pair object through native runtime");
    assert_scheme_object_export(pair);
    assert_eq!(pair.is_pair().as_result(), Ok(&true));
    assert_eq!(pair.is_list().as_result(), Ok(&false));
    assert_eq!(pair.is_null().as_result(), Ok(&false));
    let pair_head = pair.pair_car().into_result().expect("project pair car");
    let pair_tail = pair.pair_cdr().into_result().expect("project pair cdr");
    let pair_parts = pair.pair_parts().into_result().expect("project pair parts");
    assert_scheme_object_export(pair_head);
    assert_scheme_object_export(pair_tail);
    assert_eq!(pair_parts.car, pair_head);
    assert_eq!(pair_parts.cdr, pair_tail);
    assert_untrusted_raw_fail_closed(pair);
}

fn exports_proper_list_object(runtime: &GerbilRuntime) {
    let proper_list = runtime
        .fixture_proper_list_value()
        .expect("export proper Scheme list object through native runtime");
    assert_scheme_object_export(proper_list);
    assert_eq!(proper_list.is_pair().as_result(), Ok(&true));
    assert_eq!(proper_list.is_list().as_result(), Ok(&true));
    assert_eq!(proper_list.is_null().as_result(), Ok(&false));
    let proper_tail = proper_list
        .pair_cdr()
        .into_result()
        .expect("project proper-list cdr");
    assert_scheme_object_export(proper_tail);
    let proper_parts = proper_list
        .pair_parts()
        .into_result()
        .expect("project proper-list pair parts");
    assert_scheme_object_export(proper_parts.car);
    assert_eq!(proper_parts.cdr, proper_tail);
}

fn exports_improper_list_object(runtime: &GerbilRuntime) {
    let improper_list = runtime
        .fixture_improper_list_value()
        .expect("export improper Scheme list object through native runtime");
    assert_scheme_object_export(improper_list);
    assert_eq!(improper_list.is_pair().as_result(), Ok(&true));
    assert_eq!(improper_list.is_list().as_result(), Ok(&false));
    assert_eq!(improper_list.is_null().as_result(), Ok(&false));
    let improper_head = improper_list
        .pair_car()
        .into_result()
        .expect("project improper-list car");
    let improper_parts = improper_list
        .pair_parts()
        .into_result()
        .expect("project improper-list pair parts");
    assert_scheme_object_export(improper_head);
    assert_eq!(improper_parts.car, improper_head);
    assert_scheme_object_export(improper_parts.cdr);
}

fn reports_overflow_and_finalized_runtime_boundaries(runtime: GerbilRuntime) {
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

fn assert_scheme_object_export(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(
        value.provenance(),
        GerbilValueProvenance::SchemeObjectExport
    );
}

fn assert_fail_closed_value(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(value.is_pair().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_list().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.is_null().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.is_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(
        value.as_boolean().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_eq!(value.is_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.as_fixnum().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.as_fixnum_i64().status(),
        Some(GerbilStatus::InvalidValue)
    );
    assert_fail_closed_traversal(value);
}

fn assert_untrusted_raw_fail_closed(value: gerbil_scheme::GerbilValue<'_>) {
    // This deliberately re-wraps an exported non-zero handle as UntrustedRaw
    // without dereferencing it, so the test proves provenance gates reject raw
    // handles even when their bits came from live Scheme objects.
    let untrusted =
        gerbil_scheme::GerbilValue::from_raw(value.as_raw()).expect("wrap exported handle as raw");
    assert_eq!(untrusted.provenance(), GerbilValueProvenance::UntrustedRaw);
    assert_fail_closed_value(untrusted);
}

fn assert_fail_closed_traversal(value: gerbil_scheme::GerbilValue<'_>) {
    assert_eq!(value.pair_car().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(value.pair_cdr().status(), Some(GerbilStatus::InvalidValue));
    assert_eq!(
        value.pair_parts().status(),
        Some(GerbilStatus::InvalidValue)
    );
}
