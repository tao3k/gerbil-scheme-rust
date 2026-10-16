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
    let source = package.join("ffi/instance_probe.c");
    println!("cargo:rerun-if-changed={}", source.display());
    let object = out.join("instance_probe.o");
    let image = out.join("instance_probe.dylib");
    run_native_process(
        gerbil_command(gsc)
            .args(["-obj", "-cc-options", "-O2", "-o"])
            .arg(&object)
            .arg(&source),
        "compile isolated instance adapter",
        true,
    )
    .expect("instance adapter compilation");
    let compiler = discover_native_c_compiler().expect("selected host C linker");
    let mut link = Command::new(compiler.program);
    link.args(["-dynamiclib", "-o"])
        .arg(&image)
        .arg(&object)
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
    for name in ["init", "cleanup", "state", "batch", "marker"] {
        link.arg(format!(
            "-Wl,-exported_symbol,_gerbil_instance_probe_{name}"
        ));
    }
    run_native_process(&mut link, "link private runtime image", true).expect("private image link");
    println!(
        "cargo:rustc-env=GERBIL_INSTANCE_PROBE_IMAGE={}",
        image.display()
    );
}
