// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use super::{NativeLinkLibrary, ProgramManifest, native_link_options};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const PROBE: &str = "unsafe extern \"C\" { fn gerbil_dependency_probe() -> i32; } pub fn run() { assert_eq!(unsafe { gerbil_dependency_probe() }, 42); }";
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "gerbil-link-search-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    Fixture(path)
}

fn rustc() -> Command {
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
}

fn library(root: &Path, name: &str, kind: &str) -> PathBuf {
    let extension = match kind {
        "staticlib" => "a",
        "cdylib" if cfg!(target_os = "macos") => "dylib",
        "cdylib" => "so",
        _ => panic!("unsupported fixture kind"),
    };
    let symbol = if name == "gambit" {
        "gerbil_fixture_runtime"
    } else {
        "gerbil_dependency_probe"
    };
    let source = root.join(format!("{name}.rs"));
    let output = root.join(format!("lib{name}.{extension}"));
    fs::write(
        &source,
        format!("#[unsafe(no_mangle)] pub extern \"C\" fn {symbol}() -> i32 {{ 42 }}"),
    )
    .unwrap();
    let compiled = rustc()
        .args(["--edition=2024", "--crate-type", kind, "--crate-name", name])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    output
}

fn options(root: &Path, flags: Vec<String>) -> (Vec<PathBuf>, Vec<NativeLinkLibrary>) {
    native_link_options(&ProgramManifest {
        schema: String::new(),
        modules: vec![],
        stub: root.join("unused.scm"),
        library_dir: root.into(),
        link_options: flags,
    })
    .unwrap()
}

// Exercise exactly the search paths and typed modes projected for Cargo.
// Real archives replace the old text files that merely had library suffixes.
fn link(root: &Path, projected: &(Vec<PathBuf>, Vec<NativeLinkLibrary>), source: &str) -> Output {
    if !root.join("libgambit.a").is_file() {
        library(root, "gambit", "staticlib");
    }
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let input = root.join("consumer-link.rs");
    let archive = root.join(format!("libconsumer_link-{sequence}.rlib"));
    let binary = root.join(format!("consumer-{sequence}"));
    fs::write(&input, source).unwrap();
    let mut compiler = rustc();
    compiler
        .args([
            "--edition=2024",
            "--crate-type=rlib",
            "--crate-name=consumer_link",
        ])
        .arg(&input)
        .arg("-o")
        .arg(&archive);
    for directory in &projected.0 {
        compiler
            .arg("-L")
            .arg(format!("native={}", directory.display()));
    }
    for dependency in &projected.1 {
        compiler.arg("-l").arg(dependency.as_str());
    }
    let bundled = compiler.output().unwrap();
    if !bundled.status.success() {
        return bundled;
    }
    let main = root.join("consumer.rs");
    fs::write(&main, "fn main() { consumer_link::run(); }").unwrap();
    let mut linker = rustc();
    linker
        .arg("--edition=2024")
        .arg(&main)
        .arg("-o")
        .arg(&binary)
        .arg("--extern")
        .arg(format!("consumer_link={}", archive.display()));
    for directory in &projected.0 {
        linker
            .arg("-L")
            .arg(format!("native={}", directory.display()));
    }
    let linked = linker.output().unwrap();
    if linked.status.success() {
        let executed = Command::new(&binary)
            .env("LD_LIBRARY_PATH", root)
            .env("DYLD_LIBRARY_PATH", root)
            .output()
            .unwrap();
        assert!(
            executed.status.success(),
            "{}",
            String::from_utf8_lossy(&executed.stderr)
        );
    }
    linked
}

#[test]
fn native_receipt_is_projected_without_changing_link_order() {
    let root = fixture();
    let projected = options(
        &root.0,
        vec![
            format!("-L{}", root.0.join("old-cellar").display()),
            format!("-L{}", root.0.display()),
            "-lcrypto".into(),
            "-lcrypto".into(),
            "-lm".into(),
            "-ldl".into(),
        ],
    );
    assert_eq!(
        projected.0,
        [root.0.clone(), root.0.join("old-cellar"), root.0.clone()]
    );
    assert_eq!(
        projected
            .1
            .iter()
            .map(NativeLinkLibrary::as_str)
            .collect::<Vec<_>>(),
        ["static=gambit", "crypto", "crypto", "m", "dl"]
    );
}

#[test]
fn missing_dependency_is_rejected_by_the_target_linker() {
    let root = fixture();
    let projected = options(&root.0, vec!["-lgerbil_required_dependency".into()]);
    let missing = link(&root.0, &projected, PROBE);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("gerbil_required_dependency"));
    library(&root.0, "gerbil_other_dependency", "staticlib");
    assert!(!link(&root.0, &projected, PROBE).status.success());
    library(&root.0, "gerbil_required_dependency", "staticlib");
    let accepted = link(&root.0, &projected, PROBE);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
}

#[test]
fn dynamic_dependency_is_not_reinterpreted_as_a_static_archive() {
    let root = fixture();
    library(&root.0, "gerbil_mode_dependency", "cdylib");
    let dynamic = options(&root.0, vec!["-ldylib=gerbil_mode_dependency".into()]);
    let accepted = link(&root.0, &dynamic, PROBE);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let archive = options(&root.0, vec!["-lstatic=gerbil_mode_dependency".into()]);
    assert!(!link(&root.0, &archive, PROBE).status.success());
    library(&root.0, "gerbil_mode_dependency", "staticlib");
    assert!(link(&root.0, &archive, PROBE).status.success());
}

#[test]
fn implicit_system_search_is_admitted_without_inventing_a_directory() {
    let root = fixture();
    let system = if cfg!(target_os = "macos") {
        "System"
    } else {
        "c"
    };
    let projected = options(&root.0, vec![format!("-l{system}")]);
    assert_eq!(projected.0.as_slice(), std::slice::from_ref(&root.0));
    let accepted = link(
        &root.0,
        &projected,
        "unsafe extern \"C\" { fn getpid() -> i32; } pub fn run() { assert!(unsafe { getpid() } > 0); }",
    );
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
}

#[test]
fn wrong_target_library_is_rejected_by_the_target_linker() {
    let root = fixture();
    let dependency = library(&root.0, "gerbil_wrong_target", "cdylib");
    let mut bytes = fs::read(&dependency).unwrap();
    if bytes.starts_with(b"\x7fELF") {
        assert_eq!(bytes[5], 1, "little-endian fixture");
        let machine = u16::from_le_bytes(bytes[18..20].try_into().unwrap());
        let other: u16 = if machine == 62 { 183 } else { 62 };
        bytes[18..20].copy_from_slice(&other.to_le_bytes());
    } else {
        assert_eq!(&bytes[..4], &[0xcf, 0xfa, 0xed, 0xfe], "Mach-O fixture");
        let machine = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let other: u32 = if machine == 0x0100_000c {
            0x0100_0007
        } else {
            0x0100_000c
        };
        bytes[4..8].copy_from_slice(&other.to_le_bytes());
    }
    fs::write(&dependency, bytes).unwrap();
    let projected = options(&root.0, vec!["-ldylib=gerbil_wrong_target".into()]);
    let refused = link(&root.0, &projected, PROBE);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("gerbil_wrong_target"));
}
