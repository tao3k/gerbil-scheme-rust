// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

const ASP_NATIVE_SURFACE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../scheme/asp/abi-surface.ss"
));
const NATIVE_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../scheme/runtime.ss"
));
const BUILD_SCRIPT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../build.ss"));
const NATIVE_RUNTIME: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ffi/runtime.c"));

#[test]
fn fresh_utf8_entry_reencodes_in_scheme_without_snapshot_shortcuts() {
    let entry = NATIVE_SOURCE
        .split("(c-define (gerbil-rs-root-string->utf8-raw")
        .nth(1)
        .unwrap()
        .split("(c-define (gerbil-rs-root-utf8->string-raw")
        .next()
        .unwrap();
    assert!(entry.contains("(string? value)"));
    assert!(entry.contains("gerbil-rs-encode-utf8 value"));
    assert!(entry.contains("gerbil-rs-rooted-value-store!"));
    let direct = NATIVE_SOURCE
        .split("(c-define (gerbil-rs-root-string-encode-into-raw")
        .nth(1)
        .unwrap()
        .split("(c-define (gerbil-rs-root-utf8->string-raw")
        .next()
        .unwrap();
    assert!(direct.contains("(string? value)"));
    assert!(direct.contains("gerbil-rs-encode-utf8-into value pointer capacity"));
    assert!(!direct.contains("gerbil-rs-rooted-value-store!"));
    assert!(!entry.contains("string->utf8 value"));
    let encoder = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../scheme/utf8.ss"));
    assert!(encoder.contains("(##u8vector-shrink! bytes j)"));
    assert!(
        encoder.contains("end - start > 256"),
        "foreign leaf must stay bounded"
    );
    assert!(encoder.contains("___STRINGSIZE(str)"));
    assert!(encoder.contains("___U8VECTORSIZE(bytes)"));
    // A performance rollback must retain strict bulk leaf admission. These
    // assertions pin the selected recipe, not substitute for timing gates.
    assert!(encoder.contains("(make-u8vector capacity)"));
    assert!(encoder.contains("(c-lambda (scheme-object scheme-object scheme-object scheme-object scheme-object) scheme-object"));
    assert!(encoder.contains("!___FIXNUMP(first)"));
    assert!(
        !encoder.contains("malloc("),
        "heap-body views must not cross allocation"
    );
    assert!(
        !encoder.contains("table-ref"),
        "encoding must not cache mutable text"
    );
    assert!(
        !encoder.contains("string->utf8-length"),
        "no separate character length scan"
    );
}

#[test]
fn ffi_runtime_is_the_default_api_without_an_opt_in_alias() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let api = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    assert!(manifest.contains("gerbil-scheme-sys.workspace = true"));
    assert!(!manifest.contains("optional = true"));
    assert!(!manifest.contains("native ="));
    assert!(!manifest.contains("required-features"));
    assert!(!api.contains("cfg(feature"));
}

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
        BUILD_SCRIPT.contains("\"scheme/runtime\""),
        "runtime build must compile the real native implementation"
    );
    assert!(
        !BUILD_SCRIPT.contains("scheme/asp/abi-surface"),
        "ASP projection must stay out of the runtime build"
    );
}

#[test]
fn native_source_is_the_tracked_ffi_contract() {
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
        "gerbil-rs-root-string->utf8-raw",
        "gerbil-rs-root-utf8->string-raw",
        "gerbil-rs-root-bytevector-length-raw",
        "gerbil-rs-root-bytevector-copy-raw",
        "gerbil-rs-scheme-object-bytevector-copy-raw",
        "gerbil-rs-bytes->bytevector-root-raw",
        "gerbil-rs-root-release-raw",
    ] {
        assert!(
            NATIVE_SOURCE.contains(symbol),
            "native source must include stable bridge symbol {symbol}"
        );
    }
}

#[test]
fn external_program_runtime_preserves_the_sdk_vm_policy() {
    assert!(
        preserves_vm_defaults(NATIVE_RUNTIME),
        "FFI owner affinity must not override the SDK's VM processor policy"
    );
    for override_value in [1, 12] {
        let regression = format!("{NATIVE_RUNTIME}\nparams.parallelism_level = {override_value};");
        assert!(!preserves_vm_defaults(&regression));
    }
}

fn preserves_vm_defaults(source: &str) -> bool {
    source.contains("___setup_params_reset(&params);")
        && !source.contains("params.parallelism_level")
}

#[test]
fn actor_qualification_restores_the_active_vm_not_its_startup_policy() {
    let actor = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../gerbil-scheme-qualification/scheme/actors.ss"
    ));
    let restores_population = |source: &str| {
        source.contains("(original (##current-vm-processor-count))")
            && source.contains("(##cvmr original)")
            && !source.contains("(original (##get-parallelism-level))")
    };
    assert!(restores_population(actor));
    assert!(!restores_population(&actor.replace(
        "(original (##current-vm-processor-count))",
        "(original (##get-parallelism-level))"
    )));
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
