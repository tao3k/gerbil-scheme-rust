// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Owns Gerbil compilation, Gambit linking, C compilation, and archive assembly.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Build the native Gerbil and Gambit archive for the consuming Cargo package.
pub fn build_native_archive() {
    run_native_build();
}

fn run_native_build() {
    println!("cargo:rerun-if-env-changed=GERBIL_GSC");
    println!("cargo:rerun-if-env-changed=GERBIL_HOME");
    println!("cargo:rerun-if-env-changed=GERBIL_PATH");
    println!("cargo:rerun-if-changed=../../build.ss");
    println!("cargo:rerun-if-changed=../../gerbil.pkg");
    println!("cargo:rerun-if-changed=../../scheme/native.ss");
    println!("cargo:rerun-if-changed=../../native/runtime.c");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir
        .ancestors()
        .nth(2)
        .expect("sys crate must live under <workspace>/crates")
        .to_path_buf();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let gerbil_path = out_dir.join("gerbil-path");
    let gsc = env::var_os("GERBIL_GSC").unwrap_or_else(|| "gsc".into());

    let mut canonical_build = gerbil_command(workspace.join("build.ss"));
    canonical_build
        .arg("compile")
        .current_dir(&workspace)
        .env("GERBIL_PATH", &gerbil_path);
    run(&mut canonical_build, "canonical Gerbil build");

    let native_scm = gerbil_path.join("lib/static/gerbil-scheme-rust__scheme__native.scm");
    let native_c = native_scm.with_extension("c");
    let native_object = out_dir.join("native.o");
    let linker_c = out_dir.join("native_link.c");
    let linker_object = out_dir.join("native_link.o");
    let runtime_object = out_dir.join("runtime.o");

    run(
        gerbil_command(&gsc)
            .args([
                OsStr::new("-link"),
                OsStr::new("-linker-name"),
                OsStr::new("gerbil_scheme_rust_linker"),
                OsStr::new("-o"),
            ])
            .arg(&linker_c)
            .arg(&native_scm),
        "generate Gambit linker",
    );
    let compile_expression = format!(
        "(compile-file-to-target {} output: {} module-name: \"gerbil-scheme-rust/scheme/native\")",
        scheme_string(&native_scm),
        scheme_string(&native_c),
    );
    run(
        gerbil_command(&gsc).arg("-e").arg(compile_expression),
        "generate named Gerbil module C",
    );
    compile_c(
        &gsc,
        &native_c,
        &native_object,
        "compile Gerbil module object",
    );
    run(
        gerbil_command(&gsc)
            .args([
                OsStr::new("-obj"),
                OsStr::new("-cc-options"),
                OsStr::new("-Dmain=gerbil_scheme_rust_gambit_main"),
                OsStr::new("-o"),
            ])
            .arg(&linker_object)
            .arg(&linker_c),
        "compile Gambit linker",
    );
    compile_c(
        &gsc,
        &workspace.join("native/runtime.c"),
        &runtime_object,
        "compile runtime lifecycle shim",
    );

    let mut archive_build = cc::Build::new();
    archive_build
        .cargo_metadata(false)
        .out_dir(&out_dir)
        .object(&runtime_object)
        .object(&linker_object)
        .object(&native_object)
        .try_compile("gerbil_scheme_rust_native")
        .expect("archive native binding objects");

    let prefix = gerbil_prefix(&gsc);
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=gerbil_scheme_rust_native");
    println!("cargo:rustc-link-lib=static=gambit");
    println!("cargo:rustc-link-lib=dylib=m");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=dl");
    }
}

fn compile_c(gsc: &OsStr, source: &Path, object: &Path, operation: &str) {
    run(
        gerbil_command(gsc)
            .args([OsStr::new("-obj"), OsStr::new("-o")])
            .arg(object)
            .arg(source),
        operation,
    );
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

fn gerbil_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    // Nix/direnv environments commonly inject compiler include and linker
    // paths for Rust. Gerbil's selected gsc is already configured with its own
    // C compiler and SDK, so inheriting those unrelated paths can mix libc/SDK
    // headers and make even FILE or wchar_t unavailable.
    for variable in [
        "CC",
        "CFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "CPATH",
        "C_INCLUDE_PATH",
        "CPLUS_INCLUDE_PATH",
        "LIBRARY_PATH",
        "NIX_CFLAGS_COMPILE",
        "NIX_LDFLAGS",
        "SDKROOT",
    ] {
        command.env_remove(variable);
    }
    if let Some(path) = env::var_os("PATH") {
        let gerbil_path = env::join_paths(
            env::split_paths(&path).filter(|entry| !entry.starts_with("/nix/store")),
        )
        .expect("rebuild Gerbil tool PATH");
        command.env("PATH", gerbil_path);
    }
    command
}

fn run(command: &mut Command, operation: &str) -> ExitStatus {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("{operation} could not start: {error}"));
    assert!(status.success(), "{operation} failed with {status}");
    status
}
