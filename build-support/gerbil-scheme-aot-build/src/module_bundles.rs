//! Coalesce compiler-owned dynamic modules without changing logical initialization.
use crate::program::{
    cached_native_output, compile_native_object, native_build_parallelism,
    native_input_fingerprint, publish_native_output_fingerprint,
};
use crate::{
    CargoDirectiveKind, NativeArchiveLinkReceipt, ProgramArchiveObservation,
    ProgramArchiveObserver, ProgramArchiveOperation, discover_native_c_compiler, gerbil_command,
    run_native_process,
};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
/// One physical image; every logical module retains its own Gambit linker entry.
#[derive(Debug)]
pub struct NativeModuleBundle {
    pub image: PathBuf,
    pub modules: Vec<PathBuf>,
}
struct Observer;
impl ProgramArchiveObserver for Observer {
    fn observe(&self, event: ProgramArchiveObservation<'_>) {
        println!("cargo:warning=native-module {event}");
    }
}
#[derive(Debug)]
struct Module {
    source: PathBuf,
    c: PathBuf,
    object: PathBuf,
    original: PathBuf,
    group: usize,
    linker: String,
}
/// Compile generated Scheme parts using the existing native toolchain/cache.
/// The inventory is supplied by the official expander; no SDK tree is scanned.
/// Dynamic C objects default to `-O2`, matching native program objects.
/// `GERBIL_DYNAMIC_MODULE_OPT_LEVEL=0` is an explicit A/B control; changing the
/// level changes the object cache identity without changing Scheme compilation.
/// # Errors
/// Rejects unsupported targets, missing paired compiler products and native failures.
pub fn build_native_module_bundles(
    gsc: &Path,
    files: &[PathBuf],
    out: &Path,
    link: &NativeArchiveLinkReceipt,
) -> Result<Vec<NativeModuleBundle>, String> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return Err("native module groups currently require Darwin".into());
    }
    println!("cargo:rerun-if-env-changed=GERBIL_DYNAMIC_MODULE_OPT_LEVEL");
    println!("cargo:rerun-if-env-changed=GERBIL_BUILD_CORES");
    let level = std::env::var("GERBIL_DYNAMIC_MODULE_OPT_LEVEL").unwrap_or_else(|_| "2".into());
    if !["0", "1", "2", "3", "s", "g"].contains(&level.as_str()) {
        return Err(format!(
            "unsupported dynamic module optimization level {level:?}"
        ));
    }
    let modules = select_modules(files, out, &level)?;
    compile_modules(gsc, &modules, &format!("-D___DYNAMIC -O{level}"))?;
    link_groups(&modules, out, link)
}
fn select_modules(files: &[PathBuf], out: &Path, level: &str) -> Result<Vec<Module>, String> {
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut modules = Vec::new();
    for original in files {
        if !is_bundle(original)? {
            continue;
        }
        let exports = exports(original)?;
        let Some(linker) = single_linker(&exports) else {
            continue;
        };
        let name = original
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("UTF8 module filename")?
            .to_owned();
        let group = occurrences.entry(name.clone()).or_default();
        let index = *group;
        *group += 1;
        let directory = out.join("objects").join(index.to_string());
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let source = original.with_extension("scm");
        if !source.is_file() {
            return Err(format!(
                "missing compiler-owned dynamic source {}",
                source.display()
            ));
        }
        modules.push(Module {
            source,
            c: directory.join(format!("{name}.c")),
            object: directory.join(if level == "0" {
                format!("{name}.o")
            } else {
                format!("{name}.O{level}.o")
            }),
            original: original.clone(),
            group: index,
            linker: linker.to_owned(),
        });
    }
    Ok(modules)
}
fn compile_modules(gsc: &Path, modules: &[Module], options: &str) -> Result<(), String> {
    let next = AtomicUsize::new(0);
    let errors = Mutex::new(Vec::new());
    // Use the native build budget; it is independent of Scheme owner count.
    std::thread::scope(|scope| {
        for _ in 0..native_build_parallelism(modules.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(module) = modules.get(index) else {
                        break;
                    };
                    if let Err(error) = compile(gsc, module, options) {
                        errors.lock().unwrap().push(error);
                    }
                }
            });
        }
    });
    let errors = errors.into_inner().map_err(|e| e.to_string())?;
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(())
}
fn link_groups(
    modules: &[Module],
    out: &Path,
    link: &NativeArchiveLinkReceipt,
) -> Result<Vec<NativeModuleBundle>, String> {
    let images = out.join("images");
    fs::create_dir_all(&images).map_err(|e| e.to_string())?;
    let count = modules.iter().map(|m| m.group + 1).max().unwrap_or(0);
    (0..count)
        .map(|index| link_group(index, modules, &images, link))
        .collect()
}
fn link_group(
    index: usize,
    modules: &[Module],
    images: &Path,
    link: &NativeArchiveLinkReceipt,
) -> Result<NativeModuleBundle, String> {
    let members = modules
        .iter()
        .filter(|m| m.group == index)
        .collect::<Vec<_>>();
    let image = images.join(format!("group-{index}.o1"));
    let compiler = discover_native_c_compiler().map_err(|e| e.to_string())?;
    let mut command = Command::new(compiler.program);
    command
        .args(["-bundle", "-o"])
        .arg(&image)
        .args(members.iter().map(|m| &m.object));
    append_dependencies(&mut command, link);
    run_native_process(&mut command, "link dynamic module group", true)
        .map_err(|e| e.to_string())?;
    let expected = members
        .iter()
        .map(|m| m.linker.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let actual = exports(&image)?
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        return Err("coalesced image changed the native linker export set".into());
    }
    Ok(NativeModuleBundle {
        image,
        modules: members.iter().map(|m| m.original.clone()).collect(),
    })
}
fn append_dependencies(command: &mut Command, link: &NativeArchiveLinkReceipt) {
    for directive in &link.cargo_directives {
        match directive.kind {
            CargoDirectiveKind::RustcLinkSearch => {
                command.arg(format!(
                    "-L{}",
                    directive.value.trim_start_matches("native=")
                ));
            }
            CargoDirectiveKind::RustcLinkLib => {
                append_library(command, &directive.value, &link.archive_name)
            }
        }
    }
}
fn append_library(command: &mut Command, value: &str, archive: &str) {
    let name = value
        .trim_start_matches("static=")
        .trim_start_matches("dylib=");
    if [archive, "gambit", "gerbil"].contains(&name) {
        return;
    }
    command.arg(format!("-l{name}"));
}

fn compile(gsc: &Path, module: &Module, options: &str) -> Result<(), String> {
    println!("cargo:rerun-if-changed={}", module.source.display());
    println!("cargo:rerun-if-changed={}", module.original.display());
    let fingerprint = native_input_fingerprint(
        gsc,
        "dynamic-module-c-v2-preserve-module-identity",
        module
            .c
            .file_name()
            .unwrap()
            .to_str()
            .ok_or("UTF8 C output")?,
        &module.source,
    )?;
    let cached = fingerprint
        .as_deref()
        .map(|key| cached_native_output(&module.c, key))
        .transpose()?
        .unwrap_or(false);
    if !cached {
        run_native_process(
            gerbil_command(gsc)
                .args([
                    "-c",
                    "-module-ref",
                    module
                        .original
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .ok_or("UTF8 module name")?,
                    "-linker-name",
                    module
                        .original
                        .file_name()
                        .unwrap()
                        .to_str()
                        .ok_or("UTF8 linker name")?,
                    "-o",
                ])
                .arg(&module.c)
                .arg(&module.source),
            "generate dynamic module C",
            true,
        )
        .map_err(|e| e.to_string())?;
        if let Some(key) = fingerprint {
            publish_native_output_fingerprint(&module.c, &key)?;
        }
    }
    compile_native_object(
        gsc,
        &module.c,
        &module.object,
        "dynamic-module-object-v1",
        options,
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile dynamic module",
            subject: module.original.to_str(),
        },
        &Observer,
    )
}
fn is_bundle(path: &Path) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0; 16];
    if file.read(&mut header).map_err(|e| e.to_string())? < 16 {
        return Ok(false);
    }
    Ok(header[..4] == [0xcf, 0xfa, 0xed, 0xfe]
        && u32::from_le_bytes(header[12..16].try_into().unwrap()) == 8)
}

fn exports(path: &Path) -> Result<Vec<String>, String> {
    let result = Command::new("/usr/bin/nm")
        .args(["-gU"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "inspect native module exports: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(String::from_utf8(result.stdout)
        .map_err(|e| e.to_string())?
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_owned))
        .collect())
}
fn single_linker(exports: &[String]) -> Option<&str> {
    match exports {
        [symbol] if symbol.starts_with("____LNK_") => Some(symbol),
        _ => None,
    }
}
