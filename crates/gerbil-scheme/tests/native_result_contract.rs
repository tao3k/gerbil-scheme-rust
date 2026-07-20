#![cfg(feature = "native")]

use gerbil_scheme::{GerbilStatus, NativeError, NativeResult};

#[test]
fn native_result_projects_success_and_error_statuses() {
    let ok = NativeResult::ok(144_i64);
    assert!(ok.is_ok());
    assert!(!ok.is_err());
    assert_eq!(ok.status(), Some(GerbilStatus::Ok));
    assert_eq!(ok.as_result(), Ok(&144));
    assert_eq!(ok.into_result(), Ok(144));

    let known_error = NativeError::Status {
        operation: "test",
        code: GerbilStatus::RuntimeUnavailable.code(),
    };
    let known = NativeResult::<i64>::err(known_error);
    assert!(known.is_err());
    assert!(!known.is_ok());
    assert_eq!(known.status(), Some(GerbilStatus::RuntimeUnavailable));
    assert_eq!(known.as_result(), Err(&known_error));
    assert_eq!(known.into_result(), Err(known_error));

    let unknown_error = NativeError::Status {
        operation: "future-runtime",
        code: 42,
    };
    let unknown = NativeResult::<i64>::from_result(Err(unknown_error));
    assert_eq!(unknown.status(), None);
    assert_eq!(unknown.as_result(), Err(&unknown_error));

    let round_trip: Result<i64, NativeError> = NativeResult::from(Ok(233_i64)).into();
    assert_eq!(round_trip, Ok(233));
}
