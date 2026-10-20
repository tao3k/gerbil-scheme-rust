// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_aot_build::{
    CargoDirectiveKind, NativeInstanceImageRequest, ProgramArchiveRequest,
    build_native_instance_image, build_native_module_bundles, build_program_archive,
    discover_gambit_gsc_from_env, gerbil_command, run_native_process, source_workspace,
};
use gerbil_scheme_python::{
    ModuleClosure, ModuleRoot, ResourceTree, coalesce_module_resources, stage_module_closure,
};
use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = source_workspace().canonicalize().unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let gerbil_path = out.join("gerbil");
    let scheme = env::var_os("CARGO_FEATURE_SCHEME_BACKEND").is_some();
    let rust = env::var_os("CARGO_FEATURE_RUST_BACKEND").is_some();
    assert!(
        scheme || rust,
        "enable scheme-backend and/or rust-backend explicitly"
    );
    for name in [
        "build.ss",
        "gerbil.pkg",
        "scheme/ledger.ss",
        "scheme/plugin.ss",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(name).display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("scheme").display()
    );
    run_native_process(
        gerbil_command("gxi")
            .arg(workspace.join("build.ss"))
            .arg("compile")
            .current_dir(&workspace)
            .env("GERBIL_PATH", &gerbil_path),
        "compile shared Library bridge",
        true,
    )
    .unwrap();
    run_native_process(
        gerbil_command("gxi")
            .arg(root.join("build.ss"))
            .arg("compile")
            .current_dir(&root)
            .env("GERBIL_PATH", &gerbil_path),
        "compile independent Scheme ledger",
        true,
    )
    .unwrap();
    let closure_expression = format!(
        "(import {:?}) (gerbil-rs-stage-load-closure (list (quote :gerbil-python-ledger/scheme/ledger) (quote :gerbil-python-ledger/scheme/plugin)) {:?})",
        workspace.join("scheme/python-package.ss").to_str().unwrap(),
        out.join("load-closure.json").to_str().unwrap()
    );
    run_native_process(
        gerbil_command("gxi")
            .args(["-e", &closure_expression])
            .current_dir(&root)
            .env("GERBIL_PATH", &gerbil_path),
        "resolve official runtime and expansion closure",
        true,
    )
    .unwrap();
    let stage = format!(
        "(import {:?}) (gerbil-rs-stage-program {:?} {:?})",
        workspace.join("scheme/program-build.ss").to_str().unwrap(),
        root.join("scheme/ledger.ss").to_str().unwrap(),
        out.to_str().unwrap()
    );
    run_native_process(
        gerbil_command("gxi")
            .args(["-e", &stage])
            .current_dir(&root)
            .env("GERBIL_PATH", &gerbil_path),
        "stage compiler-owned ledger graph",
        true,
    )
    .unwrap();
    let gsc = discover_gambit_gsc_from_env().unwrap();
    let receipt = build_program_archive(ProgramArchiveRequest {
        manifest: &out.join("program.json"),
        gsc: &gsc,
        archive_name: "python_ledger",
        linker_name: "python_ledger",
        out_dir: &out.join("archive"),
    })
    .unwrap();
    for directive in &receipt.cargo_directives {
        match directive.kind {
            CargoDirectiveKind::RustcLinkSearch => {
                println!("cargo:rustc-link-search={}", directive.value)
            }
            CargoDirectiveKind::RustcLinkLib => {
                println!("cargo:rustc-link-lib={}", directive.value)
            }
        }
    }
    let native_image = if rust {
        Some(
            build_native_instance_image(NativeInstanceImageRequest {
                gsc: &gsc,
                archive: &receipt,
                linker_symbol: "___LNK_python__ledger",
                output: &out.join("native-build/ledger.dylib"),
                exports: &[
                    "gerbil_python_ledger_call",
                    "gerbil_scheme_python_configure",
                    "gerbil_scheme_python_apply",
                    "gerbil_python_ledger_configure",
                    "gerbil_python_ledger_close",
                ],
            })
            .expect("build native Scheme instance image"),
        )
    } else {
        None
    };
    let sdk = gsc.parent().unwrap().parent().unwrap();
    let staging = out.join(format!("package-{}", std::process::id()));
    let closure: ModuleClosure =
        serde_json::from_slice(&fs::read(out.join("load-closure.json")).unwrap()).unwrap();
    let groups = if rust {
        build_native_module_bundles(&gsc, &closure.files, &out.join("module-build"), &receipt)
            .expect("coalesce native module closure")
    } else {
        Vec::new()
    };
    let ledger_sources = root.join("scheme");
    let bridge_sources = workspace.join("scheme");
    let mut source_trees = vec![
        ResourceTree::complete(&ledger_sources, "_resources/sources/ledger"),
        ResourceTree::complete(&bridge_sources, "_resources/sources/bridge"),
    ];
    if let Some(image) = &native_image {
        fs::create_dir_all(out.join("native-runtime")).unwrap();
        fs::copy(image, out.join("native-runtime/ledger.dylib")).unwrap();
    }
    let runtime_tree = native_image.as_ref().map(|_| out.join("native-runtime"));
    if let Some(directory) = &runtime_tree {
        source_trees.push(ResourceTree::complete(directory, "_resources/native"));
    }
    let module_images = out.join("module-build/images");
    if !groups.is_empty() {
        source_trees.push(ResourceTree::complete(
            &module_images,
            "_resources/native-modules",
        ));
    }
    let mut manifest = stage_module_closure(
        &staging,
        &closure,
        &[
            ModuleRoot {
                source: &gerbil_path.join("lib"),
                destination: "_resources/runtime/application",
            },
            ModuleRoot {
                source: &sdk.join("lib"),
                destination: "_resources/runtime/sdk",
            },
        ],
        &source_trees,
    )
    .unwrap();
    let mappings = groups
        .iter()
        .map(|group| {
            (
                format!(
                    "_resources/native-modules/{}",
                    group.image.file_name().unwrap().to_str().unwrap()
                ),
                group.modules.clone(),
            )
        })
        .collect::<Vec<_>>();
    coalesce_module_resources(
        &staging,
        &mut manifest,
        &closure,
        &[
            ModuleRoot {
                source: &gerbil_path.join("lib"),
                destination: "_resources/runtime/application",
            },
            ModuleRoot {
                source: &sdk.join("lib"),
                destination: "_resources/runtime/sdk",
            },
        ],
        &mappings,
    )
    .unwrap();
    manifest
        .verify(&staging)
        .unwrap()
        .publish_to(&root.join("python/gerbil_ledger"))
        .unwrap();
    fs::write(
        out.join("package.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}
