// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use gerbil_scheme_build::{GerbilArtifactKind, GerbilCompiler, GerbilSource};

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
