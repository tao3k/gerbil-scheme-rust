#![cfg(feature = "native")]

use gerbil_scheme::{GerbilI64Callback, GerbilStatus, GerbilUtf8, GerbilValue, NativeError};

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
    let err = GerbilValue::from_raw(std::ptr::null_mut()).expect_err("null handle must fail");

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
    let raw = std::ptr::NonNull::<u8>::dangling()
        .as_ptr()
        .cast::<std::ffi::c_void>();
    let value = GerbilValue::from_raw(raw).expect("non-null opaque handle");

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
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_surface = manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root from gerbil-scheme crate")
        .join("scheme/asp/native-surface.ss");
    let source = std::fs::read_to_string(&native_surface)
        .unwrap_or_else(|err| panic!("read {}: {err}", native_surface.display()));

    for required in [
        "gerbil_scheme_rust_utf8_shape",
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_i64_callback_shape",
        "(borrowed-values (utf8))",
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
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let native_surface = manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root from gerbil-scheme crate")
        .join("scheme/asp/native-surface.ss");
    let source = std::fs::read_to_string(&native_surface)
        .unwrap_or_else(|err| panic!("read {}: {err}", native_surface.display()));

    let backed_shape_selectors = [
        "gerbil_scheme_rust_i64_shape",
        "gerbil_scheme_rust_bool_shape",
        "gerbil_scheme_rust_comparison_shape",
        "gerbil_scheme_rust_utf8_shape",
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_i64_callback_shape",
        "gerbil_scheme_rust_native_value_shape",
        "gerbil_scheme_rust_native_error_shape",
        "gerbil_scheme_rust_native_result_shape",
    ];

    for required in backed_shape_selectors {
        assert!(
            source.contains(required),
            "missing backed Scheme native-surface shape selector: {required}"
        );
    }

    for unsupported in [
        "gerbil_scheme_rust_nil_shape",
        "gerbil_scheme_rust_void_shape",
        "gerbil_scheme_rust_f64_shape",
        "gerbil_scheme_rust_char_shape",
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
