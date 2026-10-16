// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_python::{
    OwnedValue, ValueLimits,
    c_abi::{Error, ForeignCallable, ForeignState, RawCallable, RawState, RawValue, View},
};
use std::{collections::BTreeMap, path::PathBuf, process::Command};
type Factory = unsafe extern "C" fn(*const View, *mut RawCallable) -> i32;
type StateFactory = unsafe extern "C" fn(*const View, *mut RawState) -> i32;

#[test]
fn compiled_c_exports_own_values_closures_state_and_teardown_without_a_vm() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = std::env::temp_dir().join(format!(
        "gsr-owned-abi-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let library = directory.join(if cfg!(target_os = "macos") {
        "owned.dylib"
    } else {
        "owned.so"
    });
    let output = Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()))
        .args([
            "-std=c11", "-Wall", "-Wextra", "-Werror", "-shared", "-fPIC",
        ])
        .arg("-I")
        .arg(root.join("../../include"))
        .arg(root.join("tests/fixtures/owned_exports.c"))
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // SAFETY: this test owns the just-built producer; the library is retained
    // until all result/context owners and their aliases have been destroyed.
    unsafe {
        let library = libloading::Library::new(&library).unwrap();
        let count = library
            .get::<unsafe extern "C" fn() -> usize>(b"owned_test_live")
            .unwrap();
        for (name, size) in [
            (b"owned_test_view_size".as_slice(), size_of::<View>()),
            (b"owned_test_value_size".as_slice(), size_of::<RawValue>()),
            (
                b"owned_test_callable_size".as_slice(),
                size_of::<RawCallable>(),
            ),
            (b"owned_test_state_size".as_slice(), size_of::<RawState>()),
        ] {
            assert_eq!(
                library
                    .get::<unsafe extern "C" fn() -> usize>(name)
                    .unwrap()(),
                size
            );
        }
        check_lifecycle(&library);
        check_admission(&library);
        check_parallel_owners(&library);
        assert_eq!(count(), 0);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

unsafe fn check_lifecycle(library: &libloading::Library) {
    // SAFETY: the caller retains the exact test producer throughout this scope.
    unsafe {
        let count = library
            .get::<unsafe extern "C" fn() -> usize>(b"owned_test_live")
            .unwrap();
        let capture = *library.get::<Factory>(b"owned_test_capture").unwrap();
        let state_factory = *library.get::<StateFactory>(b"owned_test_state").unwrap();
        let value = OwnedValue::Map(BTreeMap::from([
            (
                "integer".into(),
                OwnedValue::Integer(-(num_bigint::BigInt::from(1) << 300_usize)),
            ),
            ("bytes".into(), OwnedValue::Bytes(vec![0, 255])),
            (
                "list".into(),
                OwnedValue::List(vec![
                    OwnedValue::Boolean(false),
                    OwnedValue::Text("λ\0data".into()),
                    OwnedValue::Null,
                ]),
            ),
            ("real".into(), OwnedValue::Real(-0.0)),
        ]));
        let callable = ForeignCallable::create(capture, &value).unwrap();
        let state = ForeignState::create(state_factory, &value).unwrap();
        let independent =
            ForeignState::create(state_factory, &OwnedValue::Integer(42.into())).unwrap();
        let retained = callable.call(&OwnedValue::Null).unwrap();
        assert_eq!(retained.decode(ValueLimits::default()).unwrap(), value);
        assert_eq!(
            state
                .replace(&OwnedValue::Integer(7.into()))
                .unwrap()
                .decode(ValueLimits::default())
                .unwrap(),
            value
        );
        assert_eq!(
            state
                .snapshot()
                .unwrap()
                .decode(ValueLimits::default())
                .unwrap(),
            OwnedValue::Integer(7.into())
        );
        assert_eq!(
            independent
                .snapshot()
                .unwrap()
                .decode(ValueLimits::default())
                .unwrap(),
            OwnedValue::Integer(42.into())
        );
        assert!(matches!(
            state.replace(&OwnedValue::Text("reject".into())),
            Err(Error::Status(3))
        ));
        assert_eq!(
            state
                .snapshot()
                .unwrap()
                .decode(ValueLimits::default())
                .unwrap(),
            OwnedValue::Integer(7.into())
        );
        let alias = callable.clone();
        callable.close().unwrap();
        assert!(matches!(alias.call(&OwnedValue::Null), Err(Error::Closed)));
        assert_eq!(retained.decode(ValueLimits::default()).unwrap(), value);
        drop(retained);
        state.close().unwrap();
        independent.close().unwrap();
        assert!(matches!(state.snapshot(), Err(Error::Closed)));
        assert_eq!(count(), 0);
    }
}

unsafe fn check_admission(library: &libloading::Library) {
    // SAFETY: the caller retains the exact test producer throughout this scope.
    unsafe {
        let count = library
            .get::<unsafe extern "C" fn() -> usize>(b"owned_test_live")
            .unwrap();
        let capture = *library.get::<Factory>(b"owned_test_capture").unwrap();
        let value = OwnedValue::Null;
        for export in [
            b"owned_test_bad_version".as_slice(),
            b"owned_test_requires_vm",
            b"owned_test_missing_call",
            b"owned_test_factory_error",
        ] {
            assert!(
                ForeignCallable::create(*library.get::<Factory>(export).unwrap(), &value).is_err()
            );
            assert_eq!(count(), 0);
        }
        let opaque = ForeignCallable::create(
            *library.get::<Factory>(b"owned_test_opaque").unwrap(),
            &value,
        )
        .unwrap();
        let observation = opaque.call(&OwnedValue::Null).unwrap();
        assert!(matches!(
            observation.decode(ValueLimits::default()),
            Err(Error::Invalid("unsupported value kind"))
        ));
        assert_eq!(count(), 2); // failed projection did not release or hide execution.
        opaque.close().unwrap();
        drop(observation);
        assert_eq!(count(), 0);
        let numeric = ForeignCallable::create(capture, &OwnedValue::Integer(0.into())).unwrap();
        assert_eq!(
            numeric
                .call(&OwnedValue::Null)
                .unwrap()
                .decode(ValueLimits::default())
                .unwrap(),
            OwnedValue::Integer(0.into())
        );
        numeric.close().unwrap();
        assert_eq!(count(), 0);
        let invalid = ForeignCallable::create(
            *library.get::<Factory>(b"owned_test_bad_result").unwrap(),
            &value,
        )
        .unwrap();
        assert!(matches!(invalid.call(&OwnedValue::Null), Err(Error::Abi)));
        assert_eq!(count(), 1);
        invalid.close().unwrap();
        assert_eq!(count(), 0);
    }
}

unsafe fn check_parallel_owners(library: &libloading::Library) {
    // SAFETY: the producer's code stays loaded until both owner threads join.
    let factory = unsafe { *library.get::<Factory>(b"owned_test_capture").unwrap() };
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    std::thread::scope(|scope| {
        let handles = (0..2)
            .map(|index| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let value = OwnedValue::Integer(index.into());
                    // SAFETY: the fixture transfers an independent C allocation to
                    // this thread; all calls/results/releases stay on this thread.
                    let callable = unsafe { ForeignCallable::create(factory, &value) }.unwrap();
                    barrier.wait();
                    for _ in 0..100 {
                        let result = callable.call(&OwnedValue::Null).unwrap();
                        assert_eq!(result.decode(ValueLimits::default()).unwrap(), value);
                        assert!(matches!(
                            result.decode(ValueLimits { depth: 0, nodes: 0 }),
                            Err(Error::Invalid("output structure bound"))
                        ));
                    }
                    callable.close().unwrap();
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}
