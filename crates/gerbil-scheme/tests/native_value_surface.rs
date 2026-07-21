#![cfg(feature = "native")]

use gerbil_scheme::{
    GerbilI64Callback, GerbilStatus, GerbilUtf8, GerbilValue, NativeError, NativeResult,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BackedTypeMatrixEntry {
    family: &'static str,
    scheme_selector: &'static str,
    raw_abi: &'static str,
    safe_surface: &'static str,
    ownership: &'static str,
    nullability: &'static str,
    failure_policy: &'static str,
    scenario: &'static str,
}

const BACKED_TYPE_MATRIX: &[BackedTypeMatrixEntry] = &[
    BackedTypeMatrixEntry {
        family: "status",
        scheme_selector: "gerbil_scheme_rust_status_shape",
        raw_abi: "GerbilStatus",
        safe_surface: "NativeError::status",
        ownership: "copy status code",
        nullability: "not pointer-backed",
        failure_policy: "unknown status preserves code",
        scenario: "status-contract",
    },
    BackedTypeMatrixEntry {
        family: "i64",
        scheme_selector: "gerbil_scheme_rust_i64_shape",
        raw_abi: "i64",
        safe_surface: "GerbilRuntime::{identity_i64,add_i64}",
        ownership: "copy scalar",
        nullability: "not pointer-backed",
        failure_policy: "overflow maps to IntegerOverflow",
        scenario: "native-identity-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "bool",
        scheme_selector: "gerbil_scheme_rust_bool_shape",
        raw_abi: "GerbilStatus + Rust bool return + Scheme boolean handle",
        safe_surface: "GerbilRuntime::is_even_i64 + GerbilValue::as_boolean",
        ownership: "copy scalar or runtime-borrowed Scheme object",
        nullability: "not pointer-backed",
        failure_policy: "status fail-closed before bool projection; raw provenance rejected",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "comparison",
        scheme_selector: "gerbil_scheme_rust_comparison_shape",
        raw_abi: "i32 comparison result",
        safe_surface: "GerbilRuntime::compare_i64",
        ownership: "copy scalar",
        nullability: "not pointer-backed",
        failure_policy: "invalid result maps to InvalidComparisonResult",
        scenario: "invalid-comparison-fail-closed",
    },
    BackedTypeMatrixEntry {
        family: "nil",
        scheme_selector: "gerbil_scheme_rust_nil_shape",
        raw_abi: "GerbilValueHandle",
        safe_surface: "GerbilValue::as_nil + SchemeNil",
        ownership: "runtime-borrowed Scheme object",
        nullability: "non-zero handle; distinct from null pointer",
        failure_policy: "non-null objects and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "void",
        scheme_selector: "gerbil_scheme_rust_void_shape",
        raw_abi: "GerbilValueHandle",
        safe_surface: "GerbilValue::as_void + SchemeVoid",
        ownership: "runtime-borrowed Scheme object",
        nullability: "non-zero handle; distinct from null pointer and nil",
        failure_policy: "non-void objects and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "fixnum",
        scheme_selector: "gerbil_scheme_rust_fixnum_shape",
        raw_abi: "GerbilFixnum",
        safe_surface: "GerbilValue::{is_fixnum,as_fixnum,as_fixnum_i64}",
        ownership: "runtime-borrowed Scheme object",
        nullability: "not pointer-backed",
        failure_policy: "non-fixnum and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "char",
        scheme_selector: "gerbil_scheme_rust_char_shape",
        raw_abi: "GerbilChar",
        safe_surface: "GerbilValue::{is_char,as_char}",
        ownership: "runtime-borrowed Scheme object",
        nullability: "not pointer-backed",
        failure_policy: "invalid Unicode scalar and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "flonum",
        scheme_selector: "gerbil_scheme_rust_flonum_shape",
        raw_abi: "GerbilFlonum",
        safe_surface: "GerbilValue::{is_flonum,as_flonum}",
        ownership: "runtime-borrowed Scheme object",
        nullability: "not pointer-backed",
        failure_policy: "non-flonum and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "bytevector",
        scheme_selector: "gerbil_scheme_rust_bytevector_shape",
        raw_abi: "GerbilValueHandle",
        safe_surface: "GerbilValue::as_bytevector + SchemeBytevector::to_bytestring",
        ownership: "runtime-borrowed Scheme object",
        nullability: "non-zero handle; distinct from null pointer",
        failure_policy: "non-bytevector objects and raw provenance fail closed",
        scenario: "native-runtime-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "rooted-bytes",
        scheme_selector: "gerbil_scheme_rust_rooted_bytes_shape",
        raw_abi: "GerbilRootId",
        safe_surface: "RootedSchemeString + RootedSchemeBytevector",
        ownership: "single-owner Scheme root released by Rust Drop",
        nullability: "positive root token; zero is conversion failure",
        failure_policy: "invalid input and stale roots fail closed",
        scenario: "bytevector-bytestring-round-trip",
    },
    BackedTypeMatrixEntry {
        family: "borrowed-utf8",
        scheme_selector: "gerbil_scheme_rust_utf8_shape",
        raw_abi: "GerbilBorrowedUtf8",
        safe_surface: "GerbilUtf8",
        ownership: "borrowed Rust UTF-8 bytes",
        nullability: "empty string may use null pointer with zero length",
        failure_policy: "non-UTF-8 belongs to bytevector future surface",
        scenario: "native-value-surface",
    },
    BackedTypeMatrixEntry {
        family: "opaque-value-handle",
        scheme_selector: "gerbil_scheme_rust_value_handle_shape",
        raw_abi: "GerbilValueHandle",
        safe_surface: "GerbilValue<'runtime>",
        ownership: "runtime-borrowed opaque handle",
        nullability: "zero rejected before FFI crossing",
        failure_policy: "zero maps to NullPointer status",
        scenario: "backed-value-family-surface",
    },
    BackedTypeMatrixEntry {
        family: "i64-callback",
        scheme_selector: "gerbil_scheme_rust_i64_callback_shape",
        raw_abi: "GerbilI64Callback",
        safe_surface: "GerbilI64Callback + GerbilI64CallbackAbi",
        ownership: "borrowed callback/context pair",
        nullability: "null context rejected before Rust call",
        failure_policy: "panic contained at native boundary",
        scenario: "native-value-surface",
    },
    BackedTypeMatrixEntry {
        family: "native-value",
        scheme_selector: "gerbil_scheme_rust_native_value_shape",
        raw_abi: "current backed C ABI value families",
        safe_surface: "GerbilRuntime + GerbilUtf8 + GerbilValue + GerbilI64Callback",
        ownership: "aggregate of scalar, borrowed, handle, and callback shapes",
        nullability: "explicit per concrete shape",
        failure_policy: "delegated to concrete backed value family",
        scenario: "backed-value-family-surface",
    },
    BackedTypeMatrixEntry {
        family: "native-error",
        scheme_selector: "gerbil_scheme_rust_native_error_shape",
        raw_abi: "GerbilStatus",
        safe_surface: "NativeError",
        ownership: "Rust safe-boundary error enum",
        nullability: "optional status projection",
        failure_policy: "unknown status preserves code",
        scenario: "source-surface-sync",
    },
    BackedTypeMatrixEntry {
        family: "native-result",
        scheme_selector: "gerbil_scheme_rust_native_result_shape",
        raw_abi: "GerbilStatus + native value payload",
        safe_surface: "Result<T, NativeError>",
        ownership: "ok native value or native error",
        nullability: "delegated to value/error shape",
        failure_policy: "fail-closed",
        scenario: "source-surface-sync",
    },
];

#[test]
fn public_backed_type_matrix_covers_current_native_surface() {
    let source = read_native_surface_source();
    assert_eq!(
        BACKED_TYPE_MATRIX.len(),
        17,
        "the release-auditable backed type matrix must change deliberately",
    );

    for entry in BACKED_TYPE_MATRIX {
        assert!(
            source.contains(entry.scheme_selector),
            "type matrix selector is missing from Scheme native surface: {entry:?}",
        );
        assert_non_empty_matrix_field(entry.family, "raw_abi", entry.raw_abi);
        assert_non_empty_matrix_field(entry.family, "safe_surface", entry.safe_surface);
        assert_non_empty_matrix_field(entry.family, "ownership", entry.ownership);
        assert_non_empty_matrix_field(entry.family, "nullability", entry.nullability);
        assert_non_empty_matrix_field(entry.family, "failure_policy", entry.failure_policy);
        assert_non_empty_matrix_field(entry.family, "scenario", entry.scenario);
    }

    let aggregate_shape =
        native_surface_shape_section(&source, "gerbil_scheme_rust_native_value_shape");
    for required_family in [
        "(scalar-values (i64 bool comparison status fixnum char flonum))",
        "(sentinel-values (nil void))",
        "(borrowed-values (bytevector utf8))",
        "(rooted-values (bytestring bytevector))",
        "(handle-values (runtime-handle gerbil-value-handle))",
        "(callback-values (i64-callback))",
    ] {
        assert!(
            aggregate_shape.contains(required_family),
            "native-value aggregate shape must stay aligned with the backed type matrix: {required_family}",
        );
    }
}

fn assert_non_empty_matrix_field(family: &str, field: &str, value: &str) {
    assert!(
        !value.trim().is_empty(),
        "backed type matrix field must not be empty: family={family} field={field}",
    );
}

#[test]
fn native_result_projects_status_and_preserves_unknown_codes() {
    let ok = NativeResult::ok(42_i64);
    assert!(ok.is_ok());
    assert!(!ok.is_err());
    assert_eq!(ok.status(), Some(GerbilStatus::Ok));
    assert_eq!(ok.as_result(), Ok(&42));
    assert_eq!(ok.into_result(), Ok(42));

    let known_status_error = NativeError::Status {
        operation: "native-result-status-test",
        code: GerbilStatus::NullPointer.code(),
    };
    assert_eq!(known_status_error.status(), Some(GerbilStatus::NullPointer));

    let known_status_result = NativeResult::<i64>::err(known_status_error);
    assert!(known_status_result.is_err());
    assert_eq!(
        known_status_result.status(),
        Some(GerbilStatus::NullPointer)
    );
    assert_eq!(known_status_result.as_result(), Err(&known_status_error));
    assert_eq!(known_status_result.into_result(), Err(known_status_error));

    let unknown_status_error = NativeError::Status {
        operation: "native-result-status-test",
        code: 9999,
    };
    assert_eq!(unknown_status_error.status(), None);
    assert_eq!(
        NativeResult::<()>::err(unknown_status_error).into_result(),
        Err(unknown_status_error)
    );
    assert_eq!(NativeResult::<()>::err(unknown_status_error).status(), None);

    let invalid_value_result =
        NativeResult::<()>::from_result(Err(NativeError::InvalidComparisonResult { code: 99 }));
    assert_eq!(
        invalid_value_result.status(),
        Some(GerbilStatus::InvalidValue)
    );

    let unprojected_lifecycle_result =
        NativeResult::<()>::from_result(Err(NativeError::InvalidLifecycleState { state: 42 }));
    assert_eq!(unprojected_lifecycle_result.status(), None);
}

#[test]
fn borrowed_utf8_surface_preserves_text_and_abi_bytes() {
    let text = "λ gerbil 🐹";
    let borrowed = GerbilUtf8::new(text);
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_str(), text);
    assert_eq!(borrowed.len(), text.len());
    assert!(!borrowed.is_empty());
    assert!(!abi.ptr.is_null());

    let bytes = unsafe { std::slice::from_raw_parts(abi.ptr.cast::<u8>(), abi.len) };
    assert_eq!(bytes, text.as_bytes());
}

#[test]
fn empty_borrowed_utf8_keeps_zero_length_contract() {
    let borrowed = GerbilUtf8::from("");
    let abi = borrowed.as_abi();

    assert_eq!(borrowed.as_str(), "");
    assert_eq!(borrowed.len(), 0);
    assert!(borrowed.is_empty());
    assert_eq!(abi.len, 0);
}

#[test]
fn value_handle_rejects_null_without_crossing_ffi() {
    let err = GerbilValue::from_raw(0).expect_err("zero handle must fail");

    assert_eq!(
        err,
        NativeError::Status {
            operation: "GerbilValue::from_raw",
            code: GerbilStatus::NullPointer as i32,
        }
    );
}

#[test]
fn value_handle_preserves_non_null_raw_identity_without_deref() {
    let raw = std::ptr::NonNull::<u8>::dangling().as_ptr().addr();
    let value = GerbilValue::from_raw(raw).expect("non-zero opaque handle");

    assert_eq!(value.as_raw(), raw);
}

#[test]
fn i64_callback_projects_rust_function_to_native_pair() {
    fn accept_positive(value: i64) -> GerbilStatus {
        if value > 0 {
            GerbilStatus::Ok
        } else {
            GerbilStatus::InvalidValue
        }
    }

    let callback = GerbilI64Callback::new(accept_positive);
    let abi = callback.as_abi();

    assert!(!abi.context().is_null());
    assert_eq!(
        unsafe { (abi.callback())(41, abi.context()) },
        GerbilStatus::Ok
    );
    assert_eq!(
        unsafe { (abi.callback())(0, abi.context()) },
        GerbilStatus::InvalidValue,
    );
}

#[test]
fn i64_callback_rejects_null_context_before_rust_call() {
    fn unreachable_callback(_: i64) -> GerbilStatus {
        panic!("null context must not call the Rust callback");
    }

    let callback = GerbilI64Callback::new(unreachable_callback);
    let abi = callback.as_abi();

    assert_eq!(
        unsafe { (abi.callback())(1, std::ptr::null_mut()) },
        GerbilStatus::NullPointer,
    );
}

#[test]
fn i64_callback_contains_panic_at_native_boundary() {
    fn panic_callback(_: i64) -> GerbilStatus {
        panic!("contained panic");
    }

    let callback = GerbilI64Callback::new(panic_callback);
    let abi = callback.as_abi();

    assert_eq!(
        unsafe { (abi.callback())(1, abi.context()) },
        GerbilStatus::Panic,
    );
}

#[test]
fn scheme_native_surface_projects_value_utf8_and_callback_shapes() {
    let source = read_native_surface_source();

    for required in [
        "gerbil_scheme_rust_utf8_shape",
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_i64_callback_shape",
        "(borrowed-values (bytevector utf8))",
        "(rooted-values (bytestring bytevector))",
        "(handle-values (runtime-handle gerbil-value-handle))",
        "(callback-values (i64-callback))",
        "(nullability . explicit-per-shape)",
        "(rooting . explicit-per-shape)",
        "(panic-policy . contained-as-panic-status)",
        "(gc-policy . no-gc-root-guarantee)",
    ] {
        assert!(
            source.contains(required),
            "missing Scheme native-surface contract token: {required}"
        );
    }
}

#[test]
fn scheme_native_surface_projects_all_backed_value_family_shapes() {
    let source = read_native_surface_source();

    assert_native_surface_shape_contract(
        &source,
        "gerbil_scheme_rust_native_value_shape",
        &[
            "(name . native-value)",
            "(transport . c-abi)",
            "(scalar-values (i64 bool comparison status fixnum char flonum))",
            "(borrowed-values (bytevector utf8))",
            "(handle-values (runtime-handle gerbil-value-handle))",
            "(callback-values (i64-callback))",
            "(nullability . explicit-per-shape)",
            "(rooting . explicit-per-shape)",
        ],
    );
    assert_native_surface_shape_contract(
        &source,
        "gerbil_scheme_rust_native_error_shape",
        &[
            "(name . native-error)",
            "(transport . rust-safe-boundary)",
            "(already-initialized . gerbil-status)",
            "(runtime-finalized . gerbil-status)",
            "(invalid-lifecycle-state . rust-internal)",
            "(status . gerbil-status-code-preserving)",
            "(abi-mismatch . gerbil-status)",
            "(wrong-thread . gerbil-status)",
            "(integer-overflow . gerbil-status)",
            "(invalid-comparison-result . gerbil-status)",
            "(unknown-status-policy . preserve-code)",
            "(projection . optional-gerbil-status)",
            "(display-policy . operation-context-preserving)",
        ],
    );
    assert_native_surface_shape_contract(
        &source,
        "gerbil_scheme_rust_native_result_shape",
        &[
            "(name . native-result)",
            "(ok . native-value)",
            "(error . native-error)",
            "(status-projection . optional-gerbil-status)",
            "(unknown-status-policy . preserve-code)",
            "(failure-policy . fail-closed)",
        ],
    );

    for unsupported in [
        "gerbil_scheme_rust_f64_shape",
        "gerbil_scheme_rust_symbol_shape",
        "gerbil_scheme_rust_pair_shape",
        "gerbil_scheme_rust_list_shape",
        "gerbil_scheme_rust_vector_shape",
    ] {
        assert!(
            !source.contains(unsupported),
            "unsupported Scheme native-surface selector must remain blocked until sys/safe ABI exists: {unsupported}"
        );
    }
}

fn read_native_surface_source() -> String {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_surface = manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root from gerbil-scheme crate")
        .join("scheme/asp/native-surface.ss");
    std::fs::read_to_string(&native_surface)
        .unwrap_or_else(|err| panic!("read {}: {err}", native_surface.display()))
}

fn assert_native_surface_shape_contract(source: &str, selector: &str, required_fields: &[&str]) {
    let shape = native_surface_shape_section(source, selector);
    assert!(
        shape.contains("'(native-shape"),
        "Scheme native-surface selector must project a native-shape receipt: {selector}"
    );
    for required in required_fields {
        assert!(
            shape.contains(required),
            "missing Scheme native-surface field `{required}` in selector {selector}:\n{shape}"
        );
    }
}

fn native_surface_shape_section<'a>(source: &'a str, selector: &str) -> &'a str {
    let start_marker = format!("(def {selector}");
    let start = source.find(&start_marker).unwrap_or_else(|| {
        panic!("missing backed Scheme native-surface shape selector: {selector}")
    });
    let tail = &source[start..];
    let next_shape = tail
        .get(start_marker.len()..)
        .and_then(|after_selector| after_selector.find("\n(def gerbil_scheme_rust_"))
        .map_or(tail.len(), |offset| start_marker.len() + offset);
    &tail[..next_shape]
}
