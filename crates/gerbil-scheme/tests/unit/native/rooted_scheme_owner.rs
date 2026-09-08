use super::{NativeError, RootedSchemeOwner};

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
