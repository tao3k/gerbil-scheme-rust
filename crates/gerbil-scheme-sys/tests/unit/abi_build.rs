//! Package operations are checked at the public ABI, before Scheme entry.
use crate::{
    GerbilBorrowedUtf8 as Text, GerbilRootId, GerbilStatus as Status,
    gerbil_scheme_rust_package_build, gerbil_scheme_rust_package_build_spec,
    gerbil_scheme_rust_package_clean, gerbil_scheme_rust_package_load_closure,
    gerbil_scheme_rust_package_main, gerbil_scheme_rust_package_paths,
    gerbil_scheme_rust_package_stage, gerbil_scheme_rust_package_test,
};

use std::io::Write;

#[test]
fn public_build_signatures_match_c_and_rust_callers() {
    const _: unsafe extern "C" fn(Text, *mut i32) -> Status = gerbil_scheme_rust_package_build;
    const _: unsafe extern "C" fn(Text, *mut i32) -> Status = gerbil_scheme_rust_package_clean;
    const _: unsafe extern "C" fn(Text, Text, *mut i32) -> Status = gerbil_scheme_rust_package_test;
    const _: unsafe extern "C" fn(Text, Text, *mut i32) -> Status =
        gerbil_scheme_rust_package_build_spec;
    const _: unsafe extern "C" fn(Text, Text, GerbilRootId, *mut i32) -> Status =
        gerbil_scheme_rust_package_stage;
    const _: unsafe extern "C" fn(GerbilRootId, Text, *mut i32) -> Status =
        gerbil_scheme_rust_package_load_closure;
    const _: unsafe extern "C" fn(Text, Text, *mut i32) -> Status =
        gerbil_scheme_rust_package_paths;
    const _: unsafe extern "C" fn(Text, *const Text, usize, *mut i32) -> Status =
        gerbil_scheme_rust_package_main;
    // Independently type-check the installed public C declarations, including
    // the rooted stage argument. No synthetic package or additional runner.
    let source = format!(
        "{}\n{}",
        crate::GERBIL_SCHEME_RUST_HEADER_SOURCE,
        r#"
_Static_assert(sizeof(GerbilRootId) == sizeof(int64_t), "root width");
_Static_assert(sizeof(GerbilStatus) == sizeof(int32_t), "status width");
typedef GerbilStatus (*Build)(GerbilBorrowedUtf8, int32_t *);
typedef GerbilStatus (*Pair)(GerbilBorrowedUtf8, GerbilBorrowedUtf8, int32_t *);
typedef GerbilStatus (*Stage)(GerbilBorrowedUtf8, GerbilBorrowedUtf8, GerbilRootId, int32_t *);
typedef GerbilStatus (*Closure)(GerbilRootId, GerbilBorrowedUtf8, int32_t *);
typedef GerbilStatus (*Main)(GerbilBorrowedUtf8, const GerbilBorrowedUtf8 *, size_t, int32_t *);
Build build = gerbil_scheme_rust_package_build;
Build clean = gerbil_scheme_rust_package_clean;
Pair test = gerbil_scheme_rust_package_test;
Pair spec = gerbil_scheme_rust_package_build_spec;
Pair paths = gerbil_scheme_rust_package_paths;
Stage stage = gerbil_scheme_rust_package_stage;
Closure closure = gerbil_scheme_rust_package_load_closure;
Main entry = gerbil_scheme_rust_package_main;
"#
    );
    let mut child = std::process::Command::new("cc")
        .args([
            "-std=c11",
            "-Werror=incompatible-pointer-types",
            "-fsyntax-only",
            "-x",
            "c",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("public ABI C compiler from current PATH");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn build_transport_rejects_invalid_spans_without_touching_results() {
    let bad_bytes = [0xffu8];
    let bad = Text {
        ptr: bad_bytes.as_ptr().cast(),
        len: bad_bytes.len(),
    };
    let valid = Text::from("must-not-be-opened");
    let mut result = -123;
    // SAFETY: readable spans and disjoint writable result; malformed UTF-8 is
    // rejected before native runtime entry. No runtime is initialized here.
    unsafe {
        assert_eq!(
            gerbil_scheme_rust_package_build(bad, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_test(valid, bad, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_build_spec(valid, bad, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_stage(valid, bad, GerbilRootId(0), &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_load_closure(GerbilRootId(0), bad, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_paths(valid, bad, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_main(valid, &raw const bad, 1, &raw mut result),
            Status::InvalidValue
        );
        assert_eq!(
            gerbil_scheme_rust_package_build(valid, std::ptr::null_mut()),
            Status::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_package_main(valid, std::ptr::null(), 1, &raw mut result),
            Status::NullPointer
        );
        assert_eq!(
            gerbil_scheme_rust_package_main(valid, &raw const valid, usize::MAX, &raw mut result),
            Status::InvalidValue
        );
    }
    assert_eq!(result, -123);
}

#[test]
fn valid_empty_main_argument_reaches_lifecycle_guard() {
    let directory = Text::from("must-not-be-opened");
    let argument = Text::empty();
    let mut result = -123;
    // SAFETY: valid borrows and result. The uninitialized lifecycle guard must
    // reject before any package or Scheme work, including empty argv strings.
    let status = unsafe {
        crate::gerbil_scheme_rust_package_main(directory, &raw const argument, 1, &raw mut result)
    };
    assert_eq!(status, Status::NotInitialized);
    assert_eq!(result, -123);
}

#[test]
fn native_runtime_host_retains_the_abi_codec_dependency() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let prefix = std::path::Path::new(env!("OUT_DIR")).join("gerbil-path");
    let output = std::env::temp_dir().join(format!(
        "native-abi-closure-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&output).unwrap();
    // JSON string escaping is compatible with these ordinary Scheme path
    // strings. Only integration-library source and output paths enter the expression.
    let expression = format!(
        "(import :std/make :gerbil-scheme-rust/scheme/program-build) (make '(\"scheme/runtime-host\") srcdir: {} libdir: {} optimize: #t) (gerbil-rs-stage-program {} {})",
        serde_json::to_string(workspace).unwrap(),
        serde_json::to_string(&prefix.join("lib")).unwrap(),
        serde_json::to_string(&workspace.join("scheme/runtime-host.ss")).unwrap(),
        serde_json::to_string(&output).unwrap(),
    );
    let result = std::process::Command::new("gxi")
        .args(["-e", &expression])
        .env("GERBIL_PATH", prefix)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "Native producer failed ({}); retained {}: stdout={} stderr={}",
        result.status,
        output.display(),
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("program.json")).unwrap()).unwrap();
    let modules = manifest["modules"].as_array().unwrap();
    for required in [
        "gerbil-scheme-rust/scheme/runtime",
        "gerbil-scheme-rust/scheme/utf8",
    ] {
        assert_eq!(
            modules
                .iter()
                .filter(|module| module["module"] == required)
                .count(),
            1,
            "public native ABI must initialize {required} exactly once; retained {}",
            output.display()
        );
    }
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn default_image_keeps_build_available_without_eager_compiler_initialization() {
    let path = std::path::Path::new(env!("OUT_DIR")).join("build/program.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let modules = manifest["modules"].as_array().unwrap();
    for (required, deferred) in [
        ("gerbil-scheme-rust/scheme/runtime-host", false),
        ("gerbil-scheme-rust/scheme/runtime", false),
        ("gerbil-scheme-rust/scheme/utf8", false),
    ] {
        let selected: Vec<_> = modules.iter().filter(|m| m["module"] == required).collect();
        assert_eq!(selected.len(), 1, "unique native module {required}");
        assert_eq!(
            selected[0]["deferred"].as_bool().unwrap_or(false),
            deferred,
            "native initialization boundary for {required}"
        );
    }
    for excluded in [
        "gerbil-scheme-rust/scheme/build",
        "gerbil/tools/gxtest",
        "gerbil/compiler",
    ] {
        assert!(
            !modules.iter().any(|m| m["module"] == excluded),
            "native build library must not become eager core dependency: {excluded}"
        );
    }
    let build = std::path::Path::new(env!("OUT_DIR"))
        .join("gerbil-path/lib/gerbil-scheme-rust/scheme/build.o1");
    assert!(
        build.is_file(),
        "native package product required: {}",
        build.display()
    );
}
