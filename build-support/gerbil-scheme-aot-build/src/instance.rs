//! Canonical private native image construction from an existing program archive.
use crate::{
    CargoDirectiveKind, NativeArchiveLinkReceipt, discover_native_c_compiler, gerbil_command,
    run_native_process, source_workspace,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
/// Inputs are maintainer-owned build artifacts, never Python runtime arguments.
#[derive(Debug)]
pub struct NativeInstanceImageRequest<'a> {
    pub gsc: &'a Path,
    pub archive: &'a NativeArchiveLinkReceipt,
    pub linker_symbol: &'a str,
    pub output: &'a Path,
    pub exports: &'a [&'a str],
}
/// Build an explicitly Darwin private VM image through the normal native graph.
/// # Errors
/// Reports unsupported targets, invalid native symbol names and compiler/link failures.
pub fn build_native_instance_image(
    request: NativeInstanceImageRequest<'_>,
) -> Result<PathBuf, String> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return Err("private native images require qualified Darwin support".into());
    }
    if !request
        .linker_symbol
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("invalid native linker identifier".into());
    }
    let parent = request.output.parent().ok_or("image output directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let source = parent.join("instance-adapter.c");
    let adapter = source_workspace().join("ffi/instance.c");
    std::fs::write(
        &source,
        format!(
            "#define GERBIL_INSTANCE_LINKER {}\n#include {:?}\n",
            request.linker_symbol,
            adapter.to_str().ok_or("UTF8 adapter")?
        ),
    )
    .map_err(|e| e.to_string())?;
    for file in [
        "ffi/instance.c",
        "ffi/instance.h",
        "ffi/instance_loader.h",
        "ffi/runtime.c",
        "ffi/host_io.h",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            source_workspace().join(file).display()
        );
    }
    let object = parent.join("instance-adapter.o");
    run_native_process(
        gerbil_command(request.gsc)
            .args(["-obj", "-cc-options", "-O2", "-o"])
            .arg(&object)
            .arg(&source),
        "compile owner runtime image",
        true,
    )
    .map_err(|e| e.to_string())?;
    let compiler = discover_native_c_compiler().map_err(|e| e.to_string())?;
    let mut link = Command::new(compiler.program);
    link.arg("-dynamiclib")
        .arg("-o")
        .arg(request.output)
        .arg(&object)
        .arg(&request.archive.archive_file);
    for directive in &request.archive.cargo_directives {
        match directive.kind {
            CargoDirectiveKind::RustcLinkSearch => {
                link.arg(format!(
                    "-L{}",
                    directive.value.trim_start_matches("native=")
                ));
            }
            CargoDirectiveKind::RustcLinkLib => {
                let name = directive
                    .value
                    .trim_start_matches("static=")
                    .trim_start_matches("dylib=");
                if name != request.archive.archive_name {
                    link.arg(format!("-l{name}"));
                }
            }
        }
    }
    for name in INSTANCE_EXPORTS.iter().chain(request.exports.iter()) {
        link.arg(format!("-Wl,-exported_symbol,_{name}"));
    }
    run_native_process(&mut link, "link owner runtime image", true).map_err(|e| e.to_string())?;
    Ok(request.output.to_path_buf())
}
const INSTANCE_EXPORTS: &[&str] = &[
    "gerbil_scheme_instance_abi",
    "gerbil_scheme_instance_bind_loader",
    "gerbil_scheme_instance_code_leases",
    "gerbil_scheme_instance_pending_modules",
    "gerbil_scheme_instance_release_modules",
    "gerbil_scheme_instance_setup",
    "gerbil_scheme_instance_setup_phase",
    "gerbil_scheme_instance_setup_diagnostic",
    "gerbil_scheme_instance_cleanup",
    "gerbil_scheme_instance_io_retired_bytes",
    "gerbil_scheme_instance_state",
    "gerbil_scheme_instance_usage",
    "gerbil_scheme_instance_gc_count",
    "gerbil_scheme_instance_diagnostics",
    "gerbil_scheme_rust_current_vm_processors",
    "gerbil_scheme_rust_root_count_raw",
    "gerbil_scheme_rust_take_call_error_raw",
    "gerbil_scheme_rust_bytes_to_bytevector_root_raw",
    "gerbil_scheme_rust_root_bytevector_length_raw",
    "gerbil_scheme_rust_root_bytevector_copy_raw",
    "gerbil_scheme_rust_root_release_raw",
    "gerbil_scheme_rust_numeric_create",
    "gerbil_scheme_rust_numeric_batch_create",
    "gerbil_scheme_rust_numeric_service_create",
    "gerbil_scheme_rust_numeric_size",
    "gerbil_scheme_rust_numeric_copy",
    "gerbil_scheme_rust_scalar_object",
    "gerbil_scheme_rust_object_scalar_indexed",
    "gerbil_scheme_rust_root_utf8_to_string_raw",
    "gerbil_scheme_rust_root_string_to_utf8_raw",
    "gerbil_scheme_rust_collection_create_raw",
    "gerbil_scheme_rust_collection_element_raw",
    "gerbil_scheme_rust_result_slot_take",
    "gerbil_scheme_rust_force_gc_raw",
    "gerbil_scheme_rust_object_kind",
    "gerbil_scheme_rust_bigint_create_raw",
    "gerbil_scheme_rust_bigint_size_raw",
    "gerbil_scheme_rust_bigint_copy_raw",
    "gerbil_scheme_rust_collection_length_raw",
    "gerbil_scheme_rust_collection_snapshot_raw",
    "gerbil_scheme_rust_table_create",
    "gerbil_scheme_rust_composite_snapshot",
    "gerbil_scheme_rust_object_identical",
];
