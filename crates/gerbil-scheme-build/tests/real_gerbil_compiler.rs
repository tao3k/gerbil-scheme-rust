// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::{
    fs,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use gerbil_scheme_build::{GerbilArtifactKind, GerbilCompiler, GerbilSource};
use rust_lang_project_harness::{RustScenarioBenchmarkStatus, validate_rust_scenario_benchmark};

#[test]
fn downstream_consumer_compiles_gerbil_source_to_gambit() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let output_dir = std::env::temp_dir().join(format!(
        "gerbil-scheme-rust-consumer-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&output_dir).expect("create isolated compiler output directory");

    let source = GerbilSource::new(
        "consumer-build-contract",
        "(export answer)\n(def (answer) 42)\n",
    )
    .expect("construct portable Gerbil source");
    let artifact = GerbilCompiler::default()
        .compile_to_gambit(&source, &output_dir)
        .expect("compile consumer Gerbil source with the configured gxc");

    assert_eq!(artifact.kind(), GerbilArtifactKind::GambitSource);
    assert_eq!(
        artifact.path().extension().and_then(|value| value.to_str()),
        Some("scm"),
    );
    assert!(artifact.path().is_file(), "gxc must emit the Gambit source");
    fs::remove_dir_all(&output_dir).expect("remove isolated compiler output directory");
}

#[test]
fn compiler_source_emission_stays_within_scenario_budget() {
    let scenario = validate_rust_scenario_benchmark(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit/scenarios/compiler-source-emission"),
    )
    .expect("validate the source-emission scenario benchmark contract");
    assert_eq!(scenario.status, RustScenarioBenchmarkStatus::Pass);

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let output_dir = std::env::temp_dir().join(format!(
        "gerbil-scheme-rust-emission-{}-{nonce}",
        std::process::id()
    ));
    let source = GerbilSource::new(
        "consumer-source-emission",
        "(export answer)\n(def (answer) 42)\n",
    )
    .expect("construct portable Gerbil source");

    let started = Instant::now();
    let artifact = GerbilCompiler::default()
        .emit_source(&source, &output_dir)
        .expect("emit consumer Gerbil source");

    assert_eq!(artifact.kind(), GerbilArtifactKind::ModuleSource);
    assert!(
        artifact.path().is_file(),
        "emission must create the module source"
    );
    assert!(
        started.elapsed() <= scenario.benchmark.max_total.as_duration(),
        "source-emission scenario exceeded {:?}: {:?}",
        scenario.benchmark.max_total.as_duration(),
        started.elapsed(),
    );

    fs::remove_dir_all(&output_dir).expect("remove isolated emission output directory");
}
