// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::{path::Path, time::Instant};

use gerbil_scheme::{GERBIL_SCHEME_RUST_ABI_VERSION, GerbilRuntime};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

#[test]
fn initialized_runtime_crosses_the_live_gerbil_abi() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/runtime-round-trip"),
    )
    .expect("validate the runtime scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let started = Instant::now();
    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");

    assert_eq!(
        runtime.abi_version().expect("query the live ABI version"),
        GERBIL_SCHEME_RUST_ABI_VERSION,
    );
    for value in 0..10_000 {
        assert_eq!(
            runtime
                .add_i64(value, 1)
                .expect("call the exported Gerbil procedure"),
            value + 1,
        );
        assert_eq!(
            runtime
                .is_even_i64(value)
                .expect("call the exported Gerbil predicate"),
            value % 2 == 0,
        );
        assert_eq!(
            runtime
                .compare_i64(value, value + 1)
                .expect("call the exported Gerbil comparison"),
            std::cmp::Ordering::Less,
        );
    }
    assert_eq!(
        runtime
            .compare_i64(i64::MIN, i64::MIN)
            .expect("compare equal boundary values"),
        std::cmp::Ordering::Equal,
    );
    assert_eq!(
        runtime
            .compare_i64(i64::MAX, i64::MIN)
            .expect("compare opposite boundary values"),
        std::cmp::Ordering::Greater,
    );
    let elapsed = started.elapsed();
    eprintln!(
        "scenario benchmark receipt: id=runtime-round-trip scalar_calls=30000 elapsed_ns={}",
        elapsed.as_nanos(),
    );
    assert!(
        elapsed <= scenario.benchmark.max_total.as_duration(),
        "runtime scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        elapsed,
    );
    check_object_exports(&runtime);
}

fn check_object_exports(runtime: &GerbilRuntime) {
    use gerbil_scheme::{CollectionKind, ObjectKind, SchemeScalar};
    unsafe extern "C" fn duplicate(root: i64) -> i64 {
        // SAFETY: the bound descriptor admits only a live owner-local strong input.
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_object_root_raw(
                gerbil_scheme_sys::gerbil_scheme_rust_object_word_raw(root),
            )
        }
    }
    unsafe extern "C" fn reject(_: i64) -> i64 {
        0
    }
    // SAFETY: both process-lived fixtures satisfy the strong-token export contract.
    let copy = unsafe { runtime.bind_object_export(duplicate) }.unwrap();
    let failed = unsafe { runtime.bind_object_export(reject) }.unwrap();
    let false_value = runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap();
    check_scalar_exports(runtime, &copy);
    let zero = runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap();
    let text = runtime.string_from_utf8("原生\0🦀").unwrap().into_object();
    assert_eq!(text.as_ref().kind().unwrap(), ObjectKind::String);
    for input in [&false_value, &zero, &text] {
        // SAFETY: root count and explicit GC are queried on the live owner.
        let before = unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() };
        let value = copy.call(input).into_result().unwrap();
        assert_eq!(
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() },
            before + 1
        );
        assert!(failed.call(input).into_result().is_err());
        unsafe { gerbil_scheme_sys::gerbil_scheme_rust_force_gc_raw() };
        assert_eq!(
            value.as_ref().kind().unwrap(),
            input.as_ref().kind().unwrap()
        );
        drop(value);
        assert_eq!(
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_count_raw() },
            before
        );
    }
    assert_eq!(
        copy.call(&false_value)
            .into_result()
            .unwrap()
            .as_ref()
            .scalar()
            .unwrap(),
        SchemeScalar::Boolean(false)
    );
    let echoed = copy.call(&text).into_result().unwrap();
    drop(text);
    assert_eq!(
        echoed
            .into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap(),
        "原生\0🦀"
    );
    check_table_snapshot(runtime, &false_value, &zero);
    let list = runtime
        .collection_from_objects(CollectionKind::List, &[&zero, &false_value])
        .unwrap()
        .into_object();
    let echoed = copy.call(&list).into_result().unwrap();
    drop(list);
    assert_eq!(
        echoed.into_collection(CollectionKind::List).unwrap().len(),
        2
    );
    assert!(
        copy.call(&zero)
            .into_result()
            .unwrap()
            .into_string()
            .is_err()
    );
    assert_eq!(
        runtime.void_object().unwrap().as_ref().kind().unwrap(),
        ObjectKind::Void
    );
    assert_eq!(
        runtime.nil_object().unwrap().as_ref().kind().unwrap(),
        ObjectKind::Nil
    );
}

fn check_scalar_exports(runtime: &GerbilRuntime, copy: &gerbil_scheme::LinkedObjectExport<'_>) {
    use gerbil_scheme::SchemeScalar;
    for scalar in [
        SchemeScalar::Char('\0'),
        SchemeScalar::Char('🦀'),
        SchemeScalar::Flonum(-0.0),
        SchemeScalar::Flonum(f64::INFINITY),
    ] {
        let value = runtime.scalar_object(scalar).unwrap();
        let actual = copy
            .call(&value)
            .into_result()
            .unwrap()
            .as_ref()
            .scalar()
            .unwrap();
        match (scalar, actual) {
            (SchemeScalar::Flonum(expected), SchemeScalar::Flonum(actual)) => {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
            (expected, actual) => assert_eq!(actual, expected),
        }
    }
}

fn check_table_snapshot(
    runtime: &GerbilRuntime,
    false_value: &gerbil_scheme::RootedSchemeObject<'_>,
    zero: &gerbil_scheme::RootedSchemeObject<'_>,
) {
    use gerbil_scheme::{ObjectKind, RootedObjectBatch, SchemeScalar, TableEquality};
    let key = runtime.string_from_utf8("key").unwrap().into_object();
    let entries = [(&key, false_value), (&key, zero)];
    let table = runtime
        .hash_table(TableEquality::Structural, &entries)
        .unwrap();
    assert_eq!(table.as_ref().kind().unwrap(), ObjectKind::GambitTable);
    let mut snapshot = RootedObjectBatch::default();
    table.copy_table_entries(&mut snapshot).unwrap();
    assert_eq!(snapshot.len(), 2);
    drop(table);
    drop(key);
    assert_eq!(
        snapshot.get(1).unwrap().scalar().unwrap(),
        SchemeScalar::Fixnum(0)
    );
}
