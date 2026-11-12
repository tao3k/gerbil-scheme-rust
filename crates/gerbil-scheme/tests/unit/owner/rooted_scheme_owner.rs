use super::{NativeError, NativeResult, RootedSchemeOwner, copy_owned_bytevector};

#[test]
fn owned_bytevector_copy_invokes_bulk_operation_at_most_once_at_every_size() {
    for length in [0, 1, 7, 8193, 65537, 1_000_000] {
        let mut calls = 0;
        let result =
            copy_owned_bytevector(NativeResult::ok(length), "copy-test", |output, size| {
                calls += 1;
                assert_eq!(size, length);
                assert!(size > 0, "validated empty values skip the copy call");
                for index in 0..size {
                    // SAFETY: the helper provides writable capacity for size bytes.
                    unsafe { output.add(index).write(u8::try_from(index % 251).unwrap()) };
                }
                gerbil_scheme_sys::GerbilStatus::Ok
            })
            .into_result()
            .expect("bulk copy succeeds");
        assert_eq!(calls, usize::from(length != 0));
        assert_eq!(result.len(), length);
        assert!(
            result
                .iter()
                .enumerate()
                .all(|(i, byte)| *byte == u8::try_from(i % 251).unwrap())
        );
    }
}

#[test]
fn failed_owned_copy_does_not_publish_uninitialized_storage() {
    let result = copy_owned_bytevector(NativeResult::ok(8193), "copy-test", |_, _| {
        gerbil_scheme_sys::GerbilStatus::InvalidValue
    });
    assert_eq!(
        result.status(),
        Some(gerbil_scheme_sys::GerbilStatus::InvalidValue)
    );
    let result = copy_owned_bytevector(
        NativeResult::err(NativeError::Status {
            operation: "length-test",
            code: gerbil_scheme_sys::GerbilStatus::InvalidValue as i32,
        }),
        "copy-test",
        |_, _| panic!("a failed length must not invoke the copy operation"),
    );
    assert_eq!(
        result.status(),
        Some(gerbil_scheme_sys::GerbilStatus::InvalidValue)
    );
}

#[test]
fn ok_status_with_an_invalid_root_fails_closed() {
    let error = RootedSchemeOwner::new(
        gerbil_scheme_sys::GerbilStatus::Ok,
        gerbil_scheme_sys::GerbilRootId(0),
        "rooted-owner-test",
    )
    .expect_err("an invalid root must not acquire a safe owner");

    assert!(matches!(
        error,
        NativeError::Status {
            operation: "rooted-owner-test",
            code,
        } if code == gerbil_scheme_sys::GerbilStatus::InvalidValue as i32
    ));
}

#[test]
fn non_ok_status_never_acquires_a_root_owner() {
    let error = RootedSchemeOwner::new(
        gerbil_scheme_sys::GerbilStatus::InvalidValue,
        gerbil_scheme_sys::GerbilRootId(1),
        "rooted-owner-test",
    )
    .expect_err("a failed ABI status must not acquire a safe owner");

    assert!(matches!(
        error,
        NativeError::Status {
            operation: "rooted-owner-test",
            code,
        } if code == gerbil_scheme_sys::GerbilStatus::InvalidValue as i32
    ));
}
