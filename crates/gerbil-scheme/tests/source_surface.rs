// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

const ASP_NATIVE_SURFACE: &str = include_str!("../../../scheme/asp/native-surface.ss");
const NATIVE_SIGNATURE: &str = include_str!("../../../scheme/native.ssi");
const BUILD_SCRIPT: &str = include_str!("../../../build.ss");

#[test]
fn asp_native_surface_exports_current_shape_selectors() {
    let expected_exports = [
        "gerbil_scheme_rust_abi_version",
        "gerbil_scheme_rust_runtime_init",
        "gerbil_scheme_rust_runtime_cleanup",
        "gerbil_scheme_rust_identity_i64",
        "gerbil_scheme_rust_add_i64",
        "gerbil_scheme_rust_is_even_i64",
        "gerbil_scheme_rust_compare_i64",
        "gerbil_scheme_rust_runtime_handle_shape",
        "gerbil_scheme_rust_status_shape",
        "gerbil_scheme_rust_i64_shape",
        "gerbil_scheme_rust_bool_shape",
        "gerbil_scheme_rust_comparison_shape",
        "gerbil_scheme_rust_fixnum_shape",
        "gerbil_scheme_rust_exact_integer_shape",
        "gerbil_scheme_rust_char_shape",
        "gerbil_scheme_rust_flonum_shape",
        "gerbil_scheme_rust_bytevector_shape",
        "gerbil_scheme_rust_rooted_bytes_shape",
        "gerbil_scheme_rust_integer_bytes_shape",
        "gerbil_scheme_rust_utf8_shape",
        "gerbil_scheme_rust_value_handle_shape",
        "gerbil_scheme_rust_nil_shape",
        "gerbil_scheme_rust_void_shape",
        "gerbil_scheme_rust_i64_callback_shape",
        "gerbil_scheme_rust_native_value_shape",
        "gerbil_scheme_rust_native_error_shape",
        "gerbil_scheme_rust_native_result_shape",
    ];

    let export_form = export_form(ASP_NATIVE_SURFACE);
    for symbol in expected_exports {
        assert!(
            export_form.contains(symbol),
            "ASP native surface export form must include {symbol}"
        );
        assert!(
            ASP_NATIVE_SURFACE.contains(&format!("(def {symbol}")),
            "ASP native surface must define exported selector {symbol}"
        );
    }
    assert_eq!(
        export_form.matches("gerbil_scheme_rust_").count(),
        expected_exports.len(),
        "ASP native surface export set must not drift without updating this test"
    );
}

#[test]
fn asp_native_surface_stays_out_of_runtime_build() {
    assert!(
        BUILD_SCRIPT.contains("\"scheme/native\""),
        "runtime build must compile the real native implementation"
    );
    assert!(
        !BUILD_SCRIPT.contains("scheme/asp/native-surface"),
        "ASP projection must stay out of the runtime build"
    );
}

#[test]
fn native_signature_is_the_tracked_gerbil_contract() {
    for symbol in [
        "gerbil-rs-fixture-fixnum-raw",
        "gerbil-rs-fixture-exact-integer-large-positive-raw",
        "gerbil-rs-fixture-exact-integer-large-negative-raw",
        "gerbil-rs-fixture-char-ascii-raw",
        "gerbil-rs-scheme-object-fixnum?-raw",
        "gerbil-rs-scheme-object-char?-raw",
        "gerbil-rs-scheme-object-flonum?-raw",
        "gerbil-rs-scheme-object-fixnum-value-raw",
        "gerbil-rs-scheme-object-exact-integer?-raw",
        "gerbil-rs-scheme-object-exact-integer-fits-i64?-raw",
        "gerbil-rs-scheme-object-exact-integer-fits-u64?-raw",
        "gerbil-rs-scheme-object-exact-integer-i64-value-raw",
        "gerbil-rs-scheme-object-exact-integer-u64-value-raw",
        "gerbil-rs-scheme-object-char-value-raw",
        "gerbil-rs-scheme-object-flonum-value-raw",
        "gerbil-rs-bytevector->bytestring-root-raw",
        "gerbil-rs-bytestring->bytevector-root-raw",
        "gerbil-rs-bytevector->uint-raw",
        "gerbil-rs-bytevector->sint-raw",
        "gerbil-rs-root-bytevector->uint-raw",
        "gerbil-rs-root-bytevector->sint-raw",
        "gerbil-rs-uint->bytevector-root-raw",
        "gerbil-rs-sint->bytevector-root-raw",
        "gerbil-rs-i64->exact-integer-root-raw",
        "gerbil-rs-u64->exact-integer-root-raw",
        "gerbil-rs-root-exact-integer?-raw",
        "gerbil-rs-root-exact-integer-fits-i64?-raw",
        "gerbil-rs-root-exact-integer-fits-u64?-raw",
        "gerbil-rs-root-exact-integer-i64-value-raw",
        "gerbil-rs-root-exact-integer-u64-value-raw",
        "gerbil-rs-root-string-length-raw",
        "gerbil-rs-root-bytevector-length-raw",
        "gerbil-rs-root-release-raw",
    ] {
        assert!(
            NATIVE_SIGNATURE.contains(symbol),
            "native signature must include stable bridge symbol {symbol}"
        );
    }
}

fn export_form(source: &str) -> &str {
    let start = source
        .find("(export")
        .expect("source must contain export form");
    let end = source[start..]
        .find(")\n\n")
        .expect("export form must end before file commentary");
    &source[start..=(start + end)]
}
