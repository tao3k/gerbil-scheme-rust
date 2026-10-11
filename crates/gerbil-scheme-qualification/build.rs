use std::{env, fs, path::PathBuf};

use gerbil_scheme_aot_build::{
    ProgramArchiveObservation, ProgramArchiveObserver, ProgramArchiveRequest,
    build_program_archive_observed, discover_gambit_gsc_from_env, gerbil_command,
    run_native_process, source_workspace,
};

#[path = "build/inputs.rs"]
mod inputs;
#[path = "build/instances.rs"]
mod instances;

struct Progress {
    out: PathBuf,
}

impl ProgramArchiveObserver for Progress {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        eprintln!("actor-aot: {observation}");
    }

    fn observe_source_input(&self, source: &std::path::Path) {
        // Cargo already tracks original producer sources below. Watching this
        // build's regenerated staging inputs creates a self-invalidating build.
        // SDK-owned sources/headers remain external inputs and must be tracked.
        if inputs::external_input(source, &self.out) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=GERBIL_HOME");
    println!("cargo:rerun-if-env-changed=GERBIL_GSC");
    println!("cargo:rerun-if-env-changed=GERBIL_GXI");
    println!("cargo:rerun-if-changed=scheme/actors.ss");
    println!("cargo:rerun-if-changed=scheme/utf8-controls.ss");
    println!("cargo:rerun-if-changed=scheme/utf8-inline-control.ss");
    let actor_aot = env::var_os("CARGO_FEATURE_ACTOR_AOT").is_some();
    let instance_probe = env::var_os("CARGO_FEATURE_INSTANCE_PROBE").is_some();
    if !actor_aot && !instance_probe {
        return;
    }
    let workspace = source_workspace()
        .canonicalize()
        .expect("producer workspace");
    for source in ["scheme", "build.ss", "gerbil.pkg"] {
        println!(
            "cargo:rerun-if-changed={}",
            workspace.join(source).display()
        );
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output"));
    let package = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("package"));
    let gerbil_path = out.join("gerbil-path");
    let stage = out.join("stage");
    fs::create_dir_all(&stage).expect("stage directory");
    let mut build = gerbil_command(workspace.join("build.ss"));
    build
        .arg("compile")
        .current_dir(&workspace)
        .env("GERBIL_PATH", &gerbil_path);
    run_native_process(&mut build, "actor qualification producer modules", true)
        .expect("compile producer modules");
    let source = package.join("scheme/actors.ss");
    let controls = package.join("scheme/utf8-controls.ss");
    let inline_control = package.join("scheme/utf8-inline-control.ss");
    // Debug formatting admits ordinary Cargo paths as quoted Scheme strings.
    // The package compiler adapter, not Rust, discovers the complete module graph.
    let expression = format!(
        "(import :gerbil/compiler :gerbil-scheme-rust/scheme/program-build) (compile-module {:?} [output-dir: {:?} invoke-gsc: #f optimize: #t static: #t]) (compile-module {:?} [output-dir: {:?} invoke-gsc: #f optimize: #t static: #t]) (compile-module {:?} [output-dir: {:?} invoke-gsc: #f optimize: #t static: #t]) (gerbil-rs-stage-program {:?} {:?})",
        inline_control.to_str().expect("UTF-8 inline control path"),
        gerbil_path
            .join("lib")
            .to_str()
            .expect("UTF-8 library path"),
        controls.to_str().expect("UTF-8 controls path"),
        gerbil_path
            .join("lib")
            .to_str()
            .expect("UTF-8 library path"),
        source.to_str().expect("UTF-8 source path"),
        gerbil_path
            .join("lib")
            .to_str()
            .expect("UTF-8 library path"),
        source.to_str().expect("UTF-8 source path"),
        stage.to_str().expect("UTF-8 stage path")
    );
    let mut stage_command = gerbil_command("gxi");
    stage_command
        .args(["-e", &expression])
        .current_dir(&workspace)
        .env("GERBIL_PATH", &gerbil_path);
    run_native_process(
        &mut stage_command,
        "actor qualification compiler graph",
        true,
    )
    .expect("stage compiler-owned graph");
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_observed(
        ProgramArchiveRequest {
            manifest: &stage.join("program.json"),
            gsc: &gsc,
            archive_name: "gerbil_actor_qualification",
            linker_name: "gerbil_actor_qualification",
            out_dir: &out.join("archive"),
        },
        &Progress { out: out.clone() },
    )
    .expect("full Gerbil actor program archive");
    if instance_probe {
        instances::build(&package, &out, &gsc, &receipt);
    }
    if actor_aot {
        for directive in receipt.cargo_directives {
            println!("{}", directive.line());
        }
    }
}
