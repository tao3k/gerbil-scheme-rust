// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Compile a compiler-owned AOT program manifest without inspecting Scheme text.

use crate::native::gerbil_command;
use crate::{
    NativeArchiveLinkReceipt, NativeLinkLibrary, NativeStaticLinkPlan,
    build_static_archive_from_link_plan,
};
use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

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

/// One compiler-owned transition in the linked AOT program build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramArchiveObservation<'a> {
    /// Stable native phase name.
    pub phase: &'static str,
    /// `start`, `complete`, or `failed`.
    pub state: &'static str,
    /// Human-readable operation owned by this crate.
    pub operation: &'a str,
    /// Optional module or artifact currently owned by the phase.
    pub subject: Option<&'a str>,
    /// Wall time is present for terminal transitions only.
    pub elapsed: Option<Duration>,
}

impl fmt::Display for ProgramArchiveObservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "phase={} state={} operation={}",
            self.phase, self.state, self.operation
        )?;
        if let Some(subject) = self.subject {
            write!(formatter, " subject={subject}")?;
        }
        if let Some(elapsed) = self.elapsed {
            write!(formatter, " elapsedMs={}", elapsed.as_millis())?;
        }
        Ok(())
    }
}

/// Receives phase changes from the canonical AOT builder.
///
/// Implementations decide how observations are presented. The builder never
/// invents a timer heartbeat: the last emitted `start` is the exact native
/// operation that still owns execution.
pub trait ProgramArchiveObserver {
    /// Records one build transition.
    fn observe(&self, observation: ProgramArchiveObservation<'_>);
}

struct NoProgramArchiveObserver;

impl ProgramArchiveObserver for NoProgramArchiveObserver {
    fn observe(&self, _observation: ProgramArchiveObservation<'_>) {}
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
    build_program_archive_observed(request, &NoProgramArchiveObserver)
}

/// Builds a linked AOT program while publishing exact native phase changes.
///
/// # Errors
/// Returns the same typed build errors as [`build_program_archive`].
pub fn build_program_archive_observed(
    request: ProgramArchiveRequest<'_>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<NativeArchiveLinkReceipt, String> {
    validate_linker_name(request.linker_name)?;
    let plan = read_manifest(request.manifest)?;
    observer.observe(ProgramArchiveObservation {
        phase: "program-plan",
        state: "complete",
        operation: "validate compiler-owned AOT manifest",
        subject: None,
        elapsed: None,
    });
    let staged = stage_program(&plan, request, observer)?;
    let link_plan = compile_program(&plan, staged, request, observer)?;
    observed_operation(
        observer,
        "static-archive",
        "package linked AOT archive",
        None,
        || build_static_archive_from_link_plan(request.archive_name, &link_plan, request.out_dir),
    )
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
    compile_sources: Vec<(PathBuf, PathBuf, String)>,
    linker_c: PathBuf,
    linker_object: PathBuf,
}

fn stage_program(
    plan: &ProgramManifest,
    request: ProgramArchiveRequest<'_>,
    observer: &dyn ProgramArchiveObserver,
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
            let source = generate_module_c(request.gsc, &staged, &module.module, observer)?;
            link.arg(&source);
            let object = request.out_dir.join(format!("module_{index}.o"));
            compile_sources.push((source, object.clone(), module.module.clone()));
            objects.push(object);
        }
    }
    link.arg(generate_module_c(
        request.gsc,
        &plan.stub,
        "program-stub",
        observer,
    )?);
    run(
        &mut link,
        "gsc-link",
        "generate program linker",
        None,
        observer,
    )?;
    Ok(StagedProgram {
        objects,
        compile_sources,
        linker_c,
        linker_object,
    })
}

fn generate_module_c(
    gsc: &Path,
    scm: &Path,
    module: &str,
    observer: &dyn ProgramArchiveObserver,
) -> Result<PathBuf, String> {
    let source = scm.with_extension("c");
    // A linker-name passed while compiling SCM also names every module's
    // entry point. Compile separately so each module keeps its own identity;
    // only the final C-only link step receives the program linker name.
    run(
        gerbil_command(gsc).args(["-c", "-o"]).arg(&source).arg(scm),
        "module-c",
        "generate program module C",
        Some(module),
        observer,
    )?;
    Ok(source)
}

fn compile_program(
    plan: &ProgramManifest,
    staged: StagedProgram,
    request: ProgramArchiveRequest<'_>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<NativeStaticLinkPlan, String> {
    let StagedProgram {
        mut objects,
        compile_sources,
        linker_c,
        linker_object,
    } = staged;
    for (source, object, module) in compile_sources {
        run(
            gerbil_command(request.gsc)
                .args(["-obj", "-cc-options", "-O2", "-o"])
                .arg(&object)
                .arg(source),
            "native-object",
            "compile program module",
            Some(&module),
            observer,
        )?;
    }
    let stub_object = request.out_dir.join("program_stub.o");
    run(
        gerbil_command(request.gsc)
            .args(["-obj", "-cc-options", "-O2", "-o"])
            .arg(&stub_object)
            .arg(plan.stub.with_extension("c")),
        "native-object",
        "compile program startup",
        Some("program-stub"),
        observer,
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
        "native-object",
        "compile program linker",
        Some("program-linker"),
        observer,
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

fn run(
    command: &mut Command,
    phase: &'static str,
    operation: &str,
    subject: Option<&str>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    observed_operation(observer, phase, operation, subject, || {
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
    })
}

fn observed_operation<T>(
    observer: &dyn ProgramArchiveObserver,
    phase: &'static str,
    operation: &str,
    subject: Option<&str>,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    observer.observe(ProgramArchiveObservation {
        phase,
        state: "start",
        operation,
        subject,
        elapsed: None,
    });
    let started = Instant::now();
    let result = action();
    observer.observe(ProgramArchiveObservation {
        phase,
        state: if result.is_ok() { "complete" } else { "failed" },
        operation,
        subject,
        elapsed: Some(started.elapsed()),
    });
    result
}
