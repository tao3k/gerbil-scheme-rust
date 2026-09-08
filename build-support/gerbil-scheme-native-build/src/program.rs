// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Compile a compiler-owned AOT program manifest without inspecting Scheme text.

use crate::native::gerbil_command;
use crate::{
    NativeArchiveLinkReceipt, NativeLinkLibrary, NativeStaticLinkPlan,
    build_static_archive_from_link_plan,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Cargo-resolved source root containing `build.ss` and the Scheme modules.
///
/// Downstream build scripts use this instead of guessing a sibling checkout.
#[must_use]
pub fn source_workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramManifest {
    schema: String,
    modules: Vec<ProgramModule>,
    stub: PathBuf,
    library_dir: PathBuf,
    link_options: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramModule {
    module: String,
    scm: PathBuf,
    system: bool,
}

/// Named inputs for a compiler-owned linked program archive.
#[derive(Clone, Copy, Debug)]
pub struct ProgramArchiveRequest<'a> {
    /// Manifest emitted by the canonical Gerbil package build.
    pub manifest: &'a Path,
    /// Installed Gambit compiler selected by the consuming build.
    pub gsc: &'a Path,
    /// Cargo static archive name, without the platform prefix or suffix.
    pub archive_name: &'a str,
    /// Compiler linker identity, before Gambit's C name encoding.
    pub linker_name: &'a str,
    /// Cargo-owned directory for generated C, objects and the archive.
    pub out_dir: &'a Path,
}

/// Compile a plan emitted by `gerbil-rs-stage-program` in the package build.
/// The consumer must enable `gerbil-scheme/external-program` so the AOT graph
/// owns the bridge module while the sys crate retains lifecycle ownership.
///
/// # Errors
/// Returns an error for invalid metadata, missing artifacts, or any failed
/// compiler/archiver command. It never falls back to process-backed evaluation.
pub fn build_program_archive(
    request: ProgramArchiveRequest<'_>,
) -> Result<NativeArchiveLinkReceipt, String> {
    validate_linker_name(request.linker_name)?;
    let plan = read_manifest(request.manifest)?;
    let staged = stage_program(&plan, request)?;
    let link_plan = compile_program(&plan, staged, request)?;
    build_static_archive_from_link_plan(request.archive_name, &link_plan, request.out_dir)
}

fn validate_linker_name(linker_name: &str) -> Result<(), String> {
    if linker_name.is_empty()
        || !linker_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("invalid native linker name".into());
    }
    Ok(())
}

fn read_manifest(manifest: &Path) -> Result<ProgramManifest, String> {
    let plan: ProgramManifest =
        serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if plan.schema != "gerbil-scheme-rust.aot-program.v1"
        || plan.modules.is_empty()
        || plan.modules.len() > 1024
    {
        return Err("invalid AOT program manifest".into());
    }
    if plan
        .modules
        .iter()
        .filter(|m| m.module == "gerbil-scheme-rust/scheme/native")
        .count()
        != 1
    {
        return Err("AOT program must contain exactly one native bridge module".into());
    }
    Ok(plan)
}

struct StagedProgram {
    objects: Vec<PathBuf>,
    compile_sources: Vec<(PathBuf, PathBuf)>,
    linker_c: PathBuf,
    linker_object: PathBuf,
}

fn stage_program(
    plan: &ProgramManifest,
    request: ProgramArchiveRequest<'_>,
) -> Result<StagedProgram, String> {
    std::fs::create_dir_all(request.out_dir).map_err(|e| e.to_string())?;
    let linker_c = request.out_dir.join("program_link.c");
    let linker_object = request.out_dir.join("program_link.o");
    let mut link = gerbil_command(request.gsc);
    link.args(["-link", "-linker-name", request.linker_name, "-o"])
        .arg(&linker_c);
    let mut objects = Vec::new();
    let mut compile_sources = Vec::new();
    for (index, module) in plan.modules.iter().enumerate() {
        if !module.scm.is_file() {
            return Err(format!("missing SCM for {}", module.module));
        }
        // Gerbil compile-exe filters empty user SCM, but keeps every installed
        // system object, including its interface/link metadata.
        if !module.system
            && std::fs::metadata(&module.scm)
                .map_err(|e| e.to_string())?
                .len()
                == 0
        {
            continue;
        }
        if module.system {
            let c = module.scm.with_extension("c");
            let object = module.scm.with_extension("o");
            if !c.is_file() || !object.is_file() {
                return Err(format!(
                    "missing installed AOT object for {}",
                    module.module
                ));
            }
            link.arg(c);
            objects.push(object);
        } else {
            // Stage outside the source tree: gsc -link creates C next to SCM.
            // Preserve the compiler's basename because it determines LNK names.
            let file_name = module.scm.file_name().ok_or("SCM filename missing")?;
            let staged = request.out_dir.join(file_name);
            std::fs::copy(&module.scm, &staged).map_err(|e| e.to_string())?;
            let source = generate_module_c(request.gsc, &staged)?;
            link.arg(&source);
            let object = request.out_dir.join(format!("module_{index}.o"));
            compile_sources.push((source, object.clone()));
            objects.push(object);
        }
    }
    link.arg(generate_module_c(request.gsc, &plan.stub)?);
    run(&mut link, "generate program linker")?;
    Ok(StagedProgram {
        objects,
        compile_sources,
        linker_c,
        linker_object,
    })
}

fn generate_module_c(gsc: &Path, scm: &Path) -> Result<PathBuf, String> {
    let source = scm.with_extension("c");
    // A linker-name passed while compiling SCM also names every module's
    // entry point. Compile separately so each module keeps its own identity;
    // only the final C-only link step receives the program linker name.
    run(
        gerbil_command(gsc).args(["-c", "-o"]).arg(&source).arg(scm),
        "generate program module C",
    )?;
    Ok(source)
}

fn compile_program(
    plan: &ProgramManifest,
    staged: StagedProgram,
    request: ProgramArchiveRequest<'_>,
) -> Result<NativeStaticLinkPlan, String> {
    let StagedProgram {
        mut objects,
        compile_sources,
        linker_c,
        linker_object,
    } = staged;
    for (source, object) in compile_sources {
        run(
            gerbil_command(request.gsc)
                .args(["-obj", "-cc-options", "-O2", "-o"])
                .arg(&object)
                .arg(source),
            "compile program module",
        )?;
    }
    let stub_object = request.out_dir.join("program_stub.o");
    run(
        gerbil_command(request.gsc)
            .args(["-obj", "-cc-options", "-O2", "-o"])
            .arg(&stub_object)
            .arg(plan.stub.with_extension("c")),
        "compile program startup",
    )?;
    objects.push(stub_object);
    run(
        gerbil_command(request.gsc)
            .args([
                "-obj",
                "-cc-options",
                "-O2 -Dmain=gerbil_scheme_rust_program_main",
                "-o",
            ])
            .arg(&linker_object)
            .arg(&linker_c),
        "compile program linker",
    )?;
    let (search, libraries) = native_link_options(plan)?;
    Ok(NativeStaticLinkPlan {
        module_objects: objects,
        link_object: linker_object,
        link_search_dirs: search,
        link_libraries: libraries,
    })
}

fn native_link_options(
    plan: &ProgramManifest,
) -> Result<(Vec<PathBuf>, Vec<NativeLinkLibrary>), String> {
    let mut search = vec![plan.library_dir.clone()];
    let mut libraries = vec![NativeLinkLibrary::new("static=gambit")];
    for option in &plan.link_options {
        if let Some(path) = option.strip_prefix("-L") {
            search.push(PathBuf::from(path));
        } else if let Some(name) = option.strip_prefix("-l") {
            libraries.push(NativeLinkLibrary::new(name));
        } else {
            return Err(format!("unsupported compiler-owned link option {option:?}"));
        }
    }
    Ok((search, libraries))
}

fn run(command: &mut Command, operation: &str) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{operation}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{operation}: {}; {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
