//! Diagnostic-only private runtime image. This is not a supported executor.

use std::{path::Path, process::Command};

use gerbil_scheme_aot_build::{
    CargoDirectiveKind, NativeArchiveLinkReceipt, discover_native_c_compiler, gerbil_command,
    run_native_process,
};

pub(super) fn build(package: &Path, out: &Path, gsc: &Path, receipt: &NativeArchiveLinkReceipt) {
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("macos"),
        "private runtime image diagnostics are currently Darwin-only; do not infer portability"
    );
    build_image(
        package,
        out,
        gsc,
        receipt,
        ImageBuild {
            adapter: "instance_probe",
            artifact: "instance_probe",
            c_options: "-O2",
            environment: "GERBIL_INSTANCE_PROBE_IMAGE",
            exports: &[
                "gerbil_instance_probe_init",
                "gerbil_instance_probe_cleanup",
                "gerbil_instance_probe_state",
                "gerbil_instance_probe_batch",
                "gerbil_instance_probe_marker",
                "gerbil_instance_probe_lifecycle",
            ],
        },
    );
    if std::env::var_os("CARGO_FEATURE_INSTANCE_LIBRARY").is_some() {
        for source in [
            "../../../ffi/runtime.c",
            "../../../ffi/host_io.h",
            "../../../ffi/instance.h",
            "../../../ffi/instance_loader.h",
            "../../../ffi/instance_timer.h",
            "../../../ffi/instance.c",
            "owner_memory.h",
            "owner_wait.h",
            "owner_callbacks.h",
        ] {
            println!(
                "cargo:rerun-if-changed={}",
                package.join("ffi").join(source).display()
            );
        }
        build_image(
            package,
            out,
            gsc,
            receipt,
            ImageBuild {
                adapter: "owner_instance",
                artifact: "owner_instance",
                c_options: "-O2",
                environment: "GERBIL_OWNER_INSTANCE_IMAGE",
                exports: OWNER_EXPORTS,
            },
        );
        for (artifact, environment, c_options) in [
            (
                "owner_fail_before",
                "GERBIL_OWNER_FAIL_BEFORE_IMAGE",
                "-O2 -DGERBIL_OWNER_FAILURE_STAGE=1",
            ),
            (
                "owner_fail_after",
                "GERBIL_OWNER_FAIL_AFTER_IMAGE",
                "-O2 -DGERBIL_OWNER_FAILURE_STAGE=2",
            ),
            (
                "owner_fail_capture",
                "GERBIL_OWNER_FAIL_CAPTURE_IMAGE",
                "-O2 -DGERBIL_OWNER_CAPTURE_FAILURE=1",
            ),
        ] {
            build_image(
                package,
                out,
                gsc,
                receipt,
                ImageBuild {
                    adapter: "owner_instance",
                    artifact,
                    environment,
                    c_options,
                    exports: OWNER_EXPORTS,
                },
            );
        }
    }
}

const OWNER_EXPORTS: &[&str] = &[
    "gerbil_owner_loader_roundtrip",
    "gerbil_scheme_instance_abi",
    "gerbil_scheme_instance_bind_loader",
    "gerbil_scheme_instance_code_leases",
    "gerbil_scheme_instance_pending_modules",
    "gerbil_scheme_instance_release_modules",
    "gerbil_scheme_instance_setup",
    "gerbil_scheme_instance_cleanup",
    "gerbil_scheme_instance_io_retired_bytes",
    "gerbil_scheme_instance_state",
    "gerbil_scheme_instance_gc_count",
    "gerbil_scheme_rust_runtime_cleanup",
    "gerbil_scheme_rust_current_vm_processors",
    "gerbil_scheme_rust_root_count_raw",
    "gerbil_scheme_rust_bytes_to_bytevector_root_raw",
    "gerbil_scheme_rust_root_bytevector_length_raw",
    "gerbil_scheme_rust_root_bytevector_copy_raw",
    "gerbil_scheme_rust_root_release_raw",
    "gerbil_owner_json_call",
    "gerbil_scheme_rust_numeric_create",
    "gerbil_scheme_rust_numeric_batch_create",
    "gerbil_scheme_rust_numeric_service_create",
    "gerbil_scheme_rust_numeric_service_enter",
    "gerbil_scheme_rust_composite_create",
    "gerbil_scheme_rust_numeric_batch_snapshot",
    "gerbil_scheme_rust_numeric_batch_copy",
    "gerbil_scheme_rust_numeric_size",
    "gerbil_scheme_rust_numeric_copy",
    "gerbil_scheme_rust_scalar_object",
    "gerbil_scheme_rust_object_scalar_indexed",
    "gerbil_scheme_rust_root_utf8_to_string_raw",
    "gerbil_scheme_rust_root_string_to_utf8_raw",
    "gerbil_scheme_rust_collection_create_raw",
    "gerbil_scheme_rust_collection_element_raw",
    "gerbil_scheme_rust_result_slot_take",
    "gerbil_scheme_rust_object_kind",
    "gerbil_scheme_rust_bigint_create_raw",
    "gerbil_scheme_rust_bigint_size_raw",
    "gerbil_scheme_rust_bigint_copy_raw",
    "gerbil_scheme_rust_collection_length_raw",
    "gerbil_scheme_rust_collection_snapshot_raw",
    "gerbil_scheme_rust_table_create",
    "gerbil_scheme_rust_composite_snapshot",
    "gerbil_scheme_rust_object_identical",
    "gerbil_scheme_rust_force_gc_raw",
    "gerbil_scheme_instance_usage",
    "gerbil_scheme_instance_diagnostics",
    "gerbil_scheme_instance_setup_phase",
    "gerbil_scheme_instance_setup_diagnostic",
    "gerbil_scheme_rust_take_call_error_raw",
    "gerbil_owner_process_resources",
    "gerbil_owner_allocator_relief",
    "gerbil_owner_memory_regions",
    "gerbil_owner_tls_available",
    "gerbil_owner_callback_tls_start",
    "gerbil_owner_callback_tls_retire",
    "gerbil_owner_callback",
    "gerbil_owner_failed_finalizer",
    "gerbil_owner_runtime_state",
    "gerbil_owner_blocking_tls_start",
    "gerbil_owner_blocking_tls_retire",
    "gerbil_owner_native_call",
    "gerbil_owner_orders_call",
    "gerbil_owner_service_call",
    "gerbil_owner_service_reference",
    "gerbil_owner_service_observe",
    "gerbil_owner_service_fail",
    "gerbil_owner_service_single",
];

#[derive(Clone, Copy)]
struct ImageBuild<'a> {
    adapter: &'a str,
    artifact: &'a str,
    c_options: &'a str,
    environment: &'a str,
    exports: &'a [&'a str],
}
fn build_image(
    package: &Path,
    out: &Path,
    gsc: &Path,
    receipt: &NativeArchiveLinkReceipt,
    config: ImageBuild<'_>,
) {
    let source = package.join(format!("ffi/{}.c", config.adapter));
    println!("cargo:rerun-if-changed={}", source.display());
    let object = out.join(format!("{}.o", config.artifact));
    let image = out.join(format!("{}.dylib", config.artifact));
    run_native_process(
        gerbil_command(gsc)
            .args(["-obj", "-cc-options", config.c_options, "-o"])
            .arg(&object)
            .arg(&source),
        "compile isolated instance adapter",
        true,
    )
    .expect("instance adapter compilation");
    let compiler = discover_native_c_compiler().expect("selected host C linker");
    let marker_source = package.join("ffi/owner_signpost.c");
    println!("cargo:rerun-if-changed={}", marker_source.display());
    let marker_object = out.join("owner_signpost.o");
    run_native_process(
        Command::new(&compiler.program)
            .args(["-O2", "-c"])
            .arg(&marker_source)
            .arg("-o")
            .arg(&marker_object),
        "compile qualification clock marker",
        true,
    )
    .expect("qualification clock marker compilation");
    let mut link = Command::new(compiler.program);
    link.args(["-dynamiclib", "-o"])
        .arg(&image)
        .arg(&object)
        .arg(&marker_object)
        .arg(&receipt.archive_file);
    for directive in &receipt.cargo_directives {
        match directive.kind {
            CargoDirectiveKind::RustcLinkSearch => {
                link.arg(format!(
                    "-L{}",
                    directive.value.trim_start_matches("native=")
                ));
            }
            CargoDirectiveKind::RustcLinkLib => {
                let library = directive
                    .value
                    .trim_start_matches("static=")
                    .trim_start_matches("dylib=");
                if library != receipt.archive_name {
                    link.arg(format!("-l{library}"));
                }
            }
        }
    }
    // Darwin's two-level namespace and a minimal export list keep each image's
    // Gambit globals private. All SDK libraries must be static, not shared.
    for name in config.exports {
        link.arg(format!("-Wl,-exported_symbol,_{name}"));
    }
    run_native_process(&mut link, "link private runtime image", true).expect("private image link");
    println!("cargo:rustc-env={}={}", config.environment, image.display());
}
