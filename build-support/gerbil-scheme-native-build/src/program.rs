// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

//! Compile a compiler-owned AOT program manifest without inspecting Scheme text.

use crate::native::gerbil_command;
use crate::{
    NativeArchiveLinkReceipt, NativeLinkLibrary, NativeStaticLinkPlan,
    build_static_archive_from_link_plan,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
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

/// Extension points for a downstream compiler-owned AOT program.
#[derive(Clone, Copy, Debug)]
pub struct ProgramArchiveContract<'a> {
    /// Modules that must each occur exactly once in the compiler manifest.
    pub required_modules: &'a [&'a str],
    /// C symbol used in place of Gambit's generated `main`.
    pub linker_main_symbol: &'a str,
    /// Caller-owned native objects included in the same static archive.
    pub additional_objects: &'a [PathBuf],
}

const DEFAULT_REQUIRED_MODULES: &[&str] = &["gerbil-scheme-rust/scheme/native"];

/// One compiler-owned transition in the linked AOT program build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramArchiveObservation<'a> {
    /// Stable native phase name.
    pub phase: &'static str,
    /// `start`, `complete`, `cached`, or `failed`.
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
pub trait ProgramArchiveObserver: Sync {
    /// Records one build transition.
    fn observe(&self, observation: ProgramArchiveObservation<'_>);

    /// Records one compiler-owned Scheme input consumed by the AOT graph.
    fn observe_source_input(&self, _source: &Path) {}
}

/// Named description for one observed native operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramArchiveOperation<'a> {
    /// Stable native phase name.
    pub phase: &'static str,
    /// Human-readable operation owned by the caller.
    pub operation: &'a str,
    /// Optional module or artifact owned by the operation.
    pub subject: Option<&'a str>,
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
    build_program_archive_with_contract(
        request,
        ProgramArchiveContract {
            required_modules: DEFAULT_REQUIRED_MODULES,
            linker_main_symbol: "gerbil_scheme_rust_program_main",
            additional_objects: &[],
        },
        observer,
    )
}

/// Builds a downstream AOT program through the canonical implementation.
///
/// # Errors
/// Returns an error when the requested contract, compiler manifest, native
/// command, or archive is invalid.
pub fn build_program_archive_with_contract(
    request: ProgramArchiveRequest<'_>,
    contract: ProgramArchiveContract<'_>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<NativeArchiveLinkReceipt, String> {
    validate_linker_name(request.linker_name)?;
    validate_linker_name(contract.linker_main_symbol)?;
    let plan = read_manifest(request.manifest, contract.required_modules)?;
    observer.observe(ProgramArchiveObservation {
        phase: "program-plan",
        state: "complete",
        operation: "validate compiler-owned AOT manifest",
        subject: None,
        elapsed: None,
    });
    for module in &plan.modules {
        observer.observe_source_input(&module.scm);
    }
    let staged = stage_program(&plan, request, observer)?;
    let mut link_plan = compile_program(
        &plan,
        staged,
        request,
        contract.linker_main_symbol,
        observer,
    )?;
    link_plan
        .module_objects
        .extend_from_slice(contract.additional_objects);
    observe_program_archive_operation(
        observer,
        ProgramArchiveOperation {
            phase: "static-archive",
            operation: "package linked AOT archive",
            subject: None,
        },
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

fn read_manifest(manifest: &Path, required_modules: &[&str]) -> Result<ProgramManifest, String> {
    let plan: ProgramManifest =
        serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if plan.schema != "gerbil-scheme-rust.aot-program.v1"
        || plan.modules.is_empty()
        || plan.modules.len() > 1024
    {
        return Err("invalid AOT program manifest".into());
    }
    for required in required_modules {
        if plan
            .modules
            .iter()
            .filter(|module| module.module == *required)
            .count()
            != 1
        {
            return Err(format!(
                "AOT program must contain required module {required} exactly once"
            ));
        }
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
    let mut linker_inputs = Vec::new();
    let module_count = plan.modules.len().to_string();
    observe_program_archive_operation(
        observer,
        ProgramArchiveOperation {
            phase: "module-c-batch",
            operation: "stage program module C sources",
            subject: Some(&module_count),
        },
        || {
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
                    link.arg(&c);
                    linker_inputs.push(c);
                    objects.push(object);
                } else {
                    // Stage outside the source tree: gsc -link creates C next to SCM.
                    // Preserve the compiler's basename because it determines LNK names.
                    let file_name = module.scm.file_name().ok_or("SCM filename missing")?;
                    let staged = request.out_dir.join(file_name);
                    std::fs::copy(&module.scm, &staged).map_err(|e| e.to_string())?;
                    let source = generate_module_c(request.gsc, &staged, &module.module, observer)?;
                    link.arg(&source);
                    linker_inputs.push(source.clone());
                    let object = request.out_dir.join(format!("module_{index}.o"));
                    compile_sources.push((source, object.clone(), module.module.clone()));
                    objects.push(object);
                }
            }
            Ok(())
        },
    )?;
    let stub_source = generate_module_c(request.gsc, &plan.stub, "program-stub", observer)?;
    link.arg(&stub_source);
    linker_inputs.push(stub_source);
    let linker_fingerprint = native_inputs_fingerprint(
        request.gsc,
        "program-linker-source-v1",
        request.linker_name,
        &linker_inputs,
    )?;
    let linker_cached = match linker_fingerprint.as_deref() {
        Some(fingerprint) => cached_native_output(&linker_c, fingerprint)?,
        None => false,
    };
    if linker_cached {
        observe_cached(observer, "gsc-link", "generate program linker", None);
    } else {
        run(
            &mut link,
            "gsc-link",
            "generate program linker",
            None,
            observer,
        )?;
        if let Some(fingerprint) = linker_fingerprint {
            publish_native_output_fingerprint(&linker_c, &fingerprint)?;
        }
    }
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
    let fingerprint = native_input_fingerprint(gsc, "module-c-v1", "-c -o", scm)?;
    if let Some(fingerprint) = fingerprint.as_deref() {
        if cached_native_output(&source, fingerprint)? {
            observe_cached(
                observer,
                "module-c",
                "generate program module C",
                Some(module),
            );
            return Ok(source);
        }
    }
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
    if let Some(fingerprint) = fingerprint {
        publish_native_output_fingerprint(&source, &fingerprint)?;
    }
    Ok(source)
}

fn compile_program(
    plan: &ProgramManifest,
    staged: StagedProgram,
    request: ProgramArchiveRequest<'_>,
    linker_main_symbol: &str,
    observer: &dyn ProgramArchiveObserver,
) -> Result<NativeStaticLinkPlan, String> {
    let StagedProgram {
        mut objects,
        compile_sources,
        linker_c,
        linker_object,
    } = staged;
    compile_program_modules(request.gsc, &compile_sources, observer)?;
    let stub_object = request.out_dir.join("program_stub.o");
    compile_native_object(
        request.gsc,
        &plan.stub.with_extension("c"),
        &stub_object,
        "program-startup-object-v1",
        "-O2",
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile program startup",
            subject: Some("program-stub"),
        },
        observer,
    )?;
    objects.push(stub_object);
    let linker_options = format!("-O2 -Dmain={linker_main_symbol}");
    compile_native_object(
        request.gsc,
        &linker_c,
        &linker_object,
        "program-linker-object-v1",
        &linker_options,
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile program linker",
            subject: Some("program-linker"),
        },
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

fn compile_program_modules(
    gsc: &Path,
    sources: &[(PathBuf, PathBuf, String)],
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    let worker_count = native_build_parallelism(sources.len());
    let batch = format!("jobs={} workers={worker_count}", sources.len());
    observe_program_archive_operation(
        observer,
        ProgramArchiveOperation {
            phase: "native-object-batch",
            operation: "compile program module objects",
            subject: Some(&batch),
        },
        || compile_program_modules_inner(gsc, sources, observer, worker_count),
    )
}

fn compile_program_modules_inner(
    gsc: &Path,
    sources: &[(PathBuf, PathBuf, String)],
    observer: &dyn ProgramArchiveObserver,
    worker_count: usize,
) -> Result<(), String> {
    if worker_count <= 1 {
        for (source, object, module) in sources {
            compile_program_module(gsc, source, object, module, observer)?;
        }
        return Ok(());
    }

    let next = AtomicUsize::new(0);
    let failure = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| {
                loop {
                    if failure.lock().expect("native build failure lock").is_some() {
                        return;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((source, object, module)) = sources.get(index) else {
                        return;
                    };
                    if let Err(error) =
                        compile_program_module(gsc, source, object, module, observer)
                    {
                        *failure.lock().expect("native build failure lock") = Some(error);
                        return;
                    }
                }
            });
        }
    });
    failure
        .into_inner()
        .expect("native build failure lock")
        .map_or(Ok(()), Err)
}

fn compile_program_module(
    gsc: &Path,
    source: &Path,
    object: &Path,
    module: &str,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    compile_native_object(
        gsc,
        source,
        object,
        "program-module-object-v1",
        "-O2",
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile program module",
            subject: Some(module),
        },
        observer,
    )
}

fn compile_native_object(
    gsc: &Path,
    source: &Path,
    object: &Path,
    domain: &str,
    cc_options: &str,
    operation: ProgramArchiveOperation<'_>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    let fingerprint = native_input_fingerprint(gsc, domain, cc_options, source)?;
    if let Some(fingerprint) = fingerprint.as_deref() {
        if cached_native_output(object, fingerprint)? {
            observe_cached(
                observer,
                operation.phase,
                operation.operation,
                operation.subject,
            );
            return Ok(());
        }
    }
    run(
        gerbil_command(gsc)
            .args(["-obj", "-cc-options", cc_options, "-o"])
            .arg(object)
            .arg(source),
        operation.phase,
        operation.operation,
        operation.subject,
        observer,
    )?;
    match fingerprint {
        Some(fingerprint) => publish_native_output_fingerprint(object, &fingerprint),
        None => Ok(()),
    }
}

fn native_input_fingerprint(
    compiler: &Path,
    domain: &str,
    options: &str,
    input: &Path,
) -> Result<Option<String>, String> {
    native_inputs_fingerprint(compiler, domain, options, &[input.to_path_buf()])
}

fn native_inputs_fingerprint(
    compiler: &Path,
    domain: &str,
    options: &str,
    inputs: &[PathBuf],
) -> Result<Option<String>, String> {
    let compiler = match fs::canonicalize(compiler) {
        Ok(compiler) => compiler,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "resolve native compiler {}: {error}",
                compiler.display()
            ));
        }
    };
    let compiler_metadata = fs::metadata(&compiler)
        .map_err(|error| format!("inspect native compiler {}: {error}", compiler.display()))?;
    let compiler_modified = compiler_metadata
        .modified()
        .map_err(|error| {
            format!(
                "inspect native compiler mtime {}: {error}",
                compiler.display()
            )
        })?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            format!(
                "invalid native compiler mtime {}: {error}",
                compiler.display()
            )
        })?;
    let mut hasher = Sha256::new();
    for value in [
        domain.as_bytes(),
        options.as_bytes(),
        compiler.as_os_str().as_encoded_bytes(),
        &compiler_metadata.len().to_le_bytes(),
        &compiler_modified.as_nanos().to_le_bytes(),
    ] {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value);
    }
    for input in inputs {
        let input_bytes = fs::read(input)
            .map_err(|error| format!("read native input {}: {error}", input.display()))?;
        let path = input.as_os_str().as_encoded_bytes();
        hasher.update((path.len() as u64).to_le_bytes());
        hasher.update(path);
        hasher.update((input_bytes.len() as u64).to_le_bytes());
        hasher.update(input_bytes);
    }
    Ok(Some(format!("{:x}", hasher.finalize())))
}

fn native_output_fingerprint_path(output: &Path) -> PathBuf {
    let mut path = output.as_os_str().to_owned();
    path.push(".sha256");
    PathBuf::from(path)
}

fn cached_native_output(output: &Path, fingerprint: &str) -> Result<bool, String> {
    if !output.is_file() {
        return Ok(false);
    }
    let stamp = native_output_fingerprint_path(output);
    match fs::read_to_string(&stamp) {
        Ok(actual) => Ok(actual == fingerprint),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "read native output fingerprint {}: {error}",
            stamp.display()
        )),
    }
}

fn publish_native_output_fingerprint(output: &Path, fingerprint: &str) -> Result<(), String> {
    let stamp = native_output_fingerprint_path(output);
    fs::write(&stamp, fingerprint).map_err(|error| {
        format!(
            "write native output fingerprint {}: {error}",
            stamp.display()
        )
    })
}

fn observe_cached(
    observer: &dyn ProgramArchiveObserver,
    phase: &'static str,
    operation: &str,
    subject: Option<&str>,
) {
    observer.observe(ProgramArchiveObservation {
        phase,
        state: "cached",
        operation,
        subject,
        elapsed: None,
    });
}

fn native_build_parallelism(job_count: usize) -> usize {
    let configured = std::env::var("GERBIL_BUILD_CORES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count > 0)
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(1);
    configured.min(job_count.max(1))
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
    if cfg!(target_os = "macos") {
        let runtime = discover_darwin_compiler_runtime()?;
        let directory = runtime.parent().ok_or_else(|| {
            format!(
                "Darwin compiler runtime has no parent: {}",
                runtime.display()
            )
        })?;
        search.push(directory.to_path_buf());
        libraries.push(NativeLinkLibrary::new("static=clang_rt.osx"));
    }
    Ok((search, libraries))
}

fn discover_darwin_compiler_runtime() -> Result<PathBuf, String> {
    let compiler = cc::Build::new().cargo_metadata(false).get_compiler();
    let output = compiler
        .to_command()
        .arg("-print-file-name=libclang_rt.osx.a")
        .output()
        .map_err(|error| format!("query Darwin compiler runtime: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "query Darwin compiler runtime: {}; {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let runtime = PathBuf::from(
        std::str::from_utf8(&output.stdout)
            .map_err(|error| format!("decode Darwin compiler runtime path: {error}"))?
            .trim(),
    );
    if !runtime.is_file() {
        return Err(format!(
            "Darwin compiler runtime is missing: {}",
            runtime.display()
        ));
    }
    Ok(runtime)
}

fn run(
    command: &mut Command,
    phase: &'static str,
    operation: &str,
    subject: Option<&str>,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    observe_program_archive_operation(
        observer,
        ProgramArchiveOperation {
            phase,
            operation,
            subject,
        },
        || {
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
        },
    )
}

/// Runs one extension operation under the same phase observation contract.
///
/// # Errors
/// Returns the action error after publishing a terminal `failed` transition.
pub fn observe_program_archive_operation<T>(
    observer: &dyn ProgramArchiveObserver,
    operation: ProgramArchiveOperation<'_>,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    observer.observe(ProgramArchiveObservation {
        phase: operation.phase,
        state: "start",
        operation: operation.operation,
        subject: operation.subject,
        elapsed: None,
    });
    let started = Instant::now();
    let result = action();
    observer.observe(ProgramArchiveObservation {
        phase: operation.phase,
        state: if result.is_ok() { "complete" } else { "failed" },
        operation: operation.operation,
        subject: operation.subject,
        elapsed: Some(started.elapsed()),
    });
    result
}

#[cfg(all(test, unix))]
#[path = "../tests/unit/program_cache_scenario.rs"]
mod cache_scenario;
