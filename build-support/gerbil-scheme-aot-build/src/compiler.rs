// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Owns Gerbil compilation, Gambit linking, C compilation, and archive assembly.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::generated_scm::{
    generated_scm_receipt, validate_generated_scm, workspace_input_fingerprint,
};

/// Build the native Gerbil and Gambit archive for the consuming Cargo package.
pub fn build_native_archive() {
    run_native_build();
}

fn run_native_build() {
    println!("cargo:rerun-if-env-changed=GERBIL_GSC");
    println!("cargo:rerun-if-env-changed=GERBIL_GXI");
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=GERBIL_HOME");
    println!("cargo:rerun-if-env-changed=GERBIL_PATH");
    println!("cargo:rerun-if-env-changed=GERBIL_SCHEME_RUST_UPDATE_GENERATED_SCM");
    println!("cargo:rerun-if-env-changed=GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM");
    println!("cargo:rerun-if-changed=../../build.ss");
    println!("cargo:rerun-if-changed=../../scheme/link-options.ss");
    for variable in [
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_LIBDIR",
        "PKG_CONFIG_SYSROOT_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    println!("cargo:rerun-if-changed=../../scheme/package-clean.ss");
    println!("cargo:rerun-if-changed=../../gerbil.pkg");
    println!("cargo:rerun-if-changed=../../scheme/runtime.ss");
    println!("cargo:rerun-if-changed=../../scheme/runtime-collections.ss");
    println!("cargo:rerun-if-changed=../../scheme/runtime-exchange.ss");
    println!("cargo:rerun-if-changed=../../scheme/runtime-object-kinds.ss");
    println!("cargo:rerun-if-changed=../../scheme/runtime-composites.ss");
    println!("cargo:rerun-if-changed=../../scheme/utf8.ss");
    println!("cargo:rerun-if-changed=../../scheme/generated/runtime.scm");
    println!("cargo:rerun-if-changed=../../scheme/generated/utf8.scm");
    println!("cargo:rerun-if-changed=../../scheme/generated/runtime.scm.sha256");
    println!("cargo:rerun-if-changed=../../scheme/generated/utf8.scm.sha256");
    println!("cargo:rerun-if-changed=../../ffi/runtime.c");
    println!("cargo:rerun-if-changed=../../ffi/host_io.h");
    println!("cargo:rerun-if-changed=../../ffi/host_environment.h");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir
        .ancestors()
        .nth(2)
        .expect("sys crate must live under <workspace>/crates")
        .to_path_buf();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let gerbil_path = out_dir.join("gerbil-path");
    let gsc = crate::discover_gambit_gsc_from_env()
        .expect("discover paired Gambit compiler")
        .into_os_string();
    for input in crate::discovery::gambit_sdk_inputs(Path::new(&gsc)) {
        println!("cargo:rerun-if-changed={}", input.display());
    }

    if env::var_os("CARGO_FEATURE_EXTERNAL_PROGRAM").is_some() {
        let runtime_object = out_dir.join("runtime.o");
        run(
            gerbil_command(&gsc)
                .args([
                    "-obj",
                    "-cc-options",
                    "-O2 -DGERBIL_SCHEME_RUST_EXTERNAL_PROGRAM",
                    "-o",
                ])
                .arg(&runtime_object)
                .arg(workspace.join("ffi/runtime.c")),
            "compile external program lifecycle owner",
        );
        cc::Build::new()
            .cargo_metadata(false)
            .out_dir(&out_dir)
            .object(&runtime_object)
            .try_compile("gerbil_scheme_rust_native")
            .expect("archive external program lifecycle owner");
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            gerbil_prefix(&gsc).join("lib").display()
        );
        println!("cargo:rustc-link-lib=static=gerbil_scheme_rust_native");
        println!("cargo:rustc-link-lib=static=gambit");
        return;
    }

    let mut canonical_build = gerbil_command(workspace.join("build.ss"));
    canonical_build
        .arg("compile")
        .current_dir(&workspace)
        .env("GERBIL_PATH", &gerbil_path);
    run(&mut canonical_build, "canonical Gerbil build");

    let generated_native_scm =
        gerbil_path.join("lib/static/gerbil-scheme-rust__scheme__runtime.scm");
    sync_generated_scm(&workspace, &generated_native_scm, "runtime");
    sync_generated_scm(
        &workspace,
        &gerbil_path.join("lib/static/gerbil-scheme-rust__scheme__utf8.scm"),
        "utf8",
    );
    build_default_program(&workspace, &out_dir, &gerbil_path, &gsc);
}

fn sync_generated_scm(workspace: &Path, native_scm: &Path, module: &str) -> PathBuf {
    let tracked_scm = workspace.join(format!("scheme/generated/{module}.scm"));
    let update = env::var("GERBIL_SCHEME_RUST_UPDATE_GENERATED_SCM").as_deref() == Ok("1");
    let check = env::var("GERBIL_SCHEME_RUST_CHECK_GENERATED_SCM").as_deref() == Ok("1");
    let input_fingerprint = workspace_input_fingerprint(workspace);

    let receipt_path = tracked_scm.with_extension("scm.sha256");
    if update {
        let parent = tracked_scm
            .parent()
            .expect("tracked generated SCM path must have a parent");
        fs::create_dir_all(parent).expect("create tracked generated SCM directory");
        let generated = fs::read(native_scm).expect("read generated native SCM");
        fs::write(&tracked_scm, &generated).expect("update tracked generated SCM");
        fs::write(
            &receipt_path,
            generated_scm_receipt(&generated, &input_fingerprint),
        )
        .expect("write external whole-file SCM receipt");
    }

    if check {
        let tracked = fs::read(&tracked_scm).expect("read tracked generated SCM");
        let receipt = fs::read_to_string(&receipt_path).expect("read external SCM receipt");
        validate_generated_scm(&tracked, &receipt, &input_fingerprint).unwrap_or_else(|error| {
            panic!("tracked generated SCM is invalid: {error}; run just bridge-regenerate")
        });
    }

    if update || check {
        tracked_scm
    } else {
        // Gerbil's canonical output filename encodes the complete module path.
        // Gambit's `-link` derives its expected LNK symbol from that basename,
        // while `compile-file-to-target` emits the stable short module symbol.
        // Give both operations the same short basename so ordinary Cargo builds
        // link the fresh artifact just like tracked-SCM check builds do.
        let transient_scm = native_scm
            .parent()
            .expect("generated native SCM must have a parent")
            .join(format!("{module}.scm"));
        fs::copy(native_scm, &transient_scm).expect("stage fresh native SCM with stable basename");
        transient_scm
    }
}

fn gerbil_prefix(gsc: &OsStr) -> PathBuf {
    if let Some(home) = env::var_os("GERBIL_HOME") {
        return PathBuf::from(home);
    }

    let output = Command::new(gsc).arg("-v").output().expect("run gsc -v");
    assert!(output.status.success(), "gsc -v failed");

    // gsc itself does not expose a machine-readable prefix flag. Its executable
    // is installed at <prefix>/bin/gsc; canonicalize the selected executable so
    // link discovery follows the same tool that compiled the objects.
    let selected = if Path::new(gsc).components().count() > 1 {
        PathBuf::from(gsc)
    } else {
        which(gsc).expect("locate selected gsc on PATH")
    };
    selected
        .canonicalize()
        .expect("canonicalize gsc")
        .parent()
        .and_then(Path::parent)
        .expect("gsc must be installed under <prefix>/bin")
        .to_path_buf()
}

fn which(program: &OsStr) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?).find_map(|dir| {
        let candidate = dir.join(program);
        candidate.is_file().then_some(candidate)
    })
}

fn scheme_string(path: &Path) -> String {
    let text = path.to_string_lossy();
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Keep selected Gerbil build tools isolated from unrelated Rust SDK settings.
///
/// The selected GSC retains its configured C compiler. Ambient Apple developer
/// directory overrides must not redirect that compiler's system tool shims.
pub fn gerbil_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program.as_ref());
    // Resolve on the executing host, including when the driver is precompiled.
    // Do not inherit a stale CI value or embed the library build host's count.
    command.env(
        "GERBIL_BUILD_CORES",
        crate::parallelism::host_build_parallelism().to_string(),
    );
    crate::tool_environment::configure_gerbil_compiler_environment(&mut command);
    if let Some(selected) = crate::resolve_gerbil_executable(Path::new(program.as_ref())) {
        crate::configure_gerbil_native_tool_command(&mut command, selected);
    }
    command
}

/// Add real Gambit runtime diagnostics to a Gerbil build command while
/// retaining the selected SDK's existing runtime path options.
pub fn configure_gerbil_runtime_diagnostics(command: &mut Command, enabled: bool) {
    if !enabled {
        return;
    }
    let configured = command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new("GAMBOPT"))
        .map(|(_, value)| value.map(OsStr::to_os_string));
    let mut options = configured
        .unwrap_or_else(|| env::var_os("GAMBOPT"))
        .unwrap_or_default();
    if !options.is_empty() {
        options.push(",");
    }
    options.push("1n,2n,d5qQ");
    command.env("GAMBOPT", options);
}

#[cfg(test)]
#[path = "../tests/unit/environment_scenario.rs"]
mod environment_tests;

fn run(command: &mut Command, operation: &str) {
    crate::run_native_process(command, operation, true)
        .unwrap_or_else(|error| panic!("{operation} failed: {error}"));
}

// Gerbil bootstrap only. Package operations after initialization use the native C ABI.
fn build_default_program(workspace: &Path, out: &Path, prefix: &Path, gsc: &OsStr) {
    let stage = out.join("build");
    fs::create_dir_all(&stage).expect("native Gerbil program output");
    for source in [
        "scheme/build.ss",
        "scheme/program-build.ss",
        "scheme/link-options.ss",
        "scheme/program-metadata.ss",
        "scheme/python-package.ss",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            workspace.join(source).display()
        );
    }
    let expression = format!(
        "(import :gerbil-scheme-rust/scheme/program-build) (gerbil-rs-stage-program {} {} '(:gerbil/core :gerbil/compiler :gerbil/tools/gxtest))",
        scheme_string(&workspace.join("scheme/build.ss")),
        scheme_string(&stage)
    );
    run(
        gerbil_command("gxi")
            .args(["-e", &expression])
            .env("GERBIL_PATH", prefix),
        "Gerbil build graph",
    );
    let receipt = crate::build_program_archive(crate::ProgramArchiveRequest {
        manifest: &stage.join("program.json"),
        gsc: Path::new(gsc),
        archive_name: "gerbil_scheme_build",
        linker_name: "gerbil_scheme_rust_linker",
        out_dir: &out.join("build-archive"),
    })
    .expect("Gerbil build archive");
    let home = gerbil_prefix(gsc);
    let mappings = [
        home.to_str()
            .expect("Gerbil installation UTF-8 path")
            .to_owned(),
        format!("bin={}", home.join("bin").display()),
        format!("lib={}", home.join("lib").display()),
        format!("include={}", home.join("include").display()),
    ];
    let mut header = String::new();
    for (index, value) in mappings.iter().enumerate() {
        let mut units = value
            .encode_utf16()
            .map(|unit| unit.to_string())
            .collect::<Vec<_>>();
        units.push("0".to_owned());
        header.push_str(&format!(
            "static ___UCS_2 package_toolchain_path_{index}[] = {{{}}};\n",
            units.join(",")
        ));
    }
    header.push_str("static ___UCS_2STRING package_toolchain_map[] = {package_toolchain_path_1, package_toolchain_path_2, package_toolchain_path_3, 0};\n");
    fs::write(out.join("build-home.h"), header).expect("write paired Gerbil installation mapping");
    let lifecycle_options = format!("-O2 -DGERBIL_SCHEME_RUST_BUILD -I{}", out.display());
    let lifecycle = out.join("lifecycle.o");
    run(
        gerbil_command(gsc)
            .args(["-obj", "-cc-options", &lifecycle_options, "-o"])
            .arg(&lifecycle)
            .arg(workspace.join("ffi/runtime.c")),
        "Gerbil build lifecycle",
    );
    publish_default_image(&receipt, &lifecycle, out);
}

fn publish_default_image(receipt: &crate::NativeArchiveLinkReceipt, lifecycle: &Path, out: &Path) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // Keep the complete tooling graph in its package-owned native image.
        // Rust's executable dead-strip traversal recursively walks large Scheme
        // constant graphs and can exhaust Apple's linker stack. Native image
        // assembly reuses the existing owner; no caller linker override is needed.
        let image = out.join("libgerbil_scheme_build.dylib");
        crate::instance::link_program_image(receipt, lifecycle, &image, &[], Some(&image))
            .expect("link Gerbil default build image");
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=dylib=gerbil_scheme_build");
        return;
    }
    cc::Build::new()
        .cargo_metadata(false)
        .out_dir(out)
        .object(lifecycle)
        .try_compile("gerbil_scheme_rust_native")
        .expect("archive native lifecycle");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=gerbil_scheme_rust_native");
    for directive in &receipt.cargo_directives {
        println!("{}", directive.line());
    }
}
