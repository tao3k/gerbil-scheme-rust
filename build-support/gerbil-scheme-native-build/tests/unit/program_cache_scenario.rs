// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::{ProgramArchiveObservation, ProgramArchiveObserver, compile_program_module};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

const INPUT: &str = include_str!("scenarios/native-program-content-cache/inputs/module.scm");

struct Observations(Mutex<Vec<String>>);

impl ProgramArchiveObserver for Observations {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        self.0
            .lock()
            .expect("observation lock")
            .push(observation.to_string());
    }
}

#[test]
fn unchanged_native_input_reuses_content_addressed_output() {
    let root = std::env::temp_dir().join(format!(
        "gerbil-native-content-cache-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create cache scenario root");
    let log = root.join("compiler.log");
    let compiler = root.join("fake-gsc");
    fs::write(
        &compiler,
        format!(
            "#!/bin/sh\nprintf 'invoke\\n' >> '{}'\noutput=''\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = '-o' ]; then shift; output=\"$1\"; fi\n  shift\ndone\n: > \"$output\"\n",
            log.display()
        ),
    )
    .expect("write fake compiler");
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755))
        .expect("make fake compiler executable");
    let input = root.join("module.scm");
    let output = root.join("module.o");
    fs::write(&input, INPUT).expect("write first input");
    let header = root.join("bridge.h");
    fs::write(&header, "#define BRIDGE_VERSION 1\n").expect("write header input");
    let header_files = [header.clone()];
    let header_input = [super::NativeHeaderInput {
        include_directory: &root,
        header_files: &header_files,
    }];
    let observations = Observations(Mutex::new(Vec::new()));
    let options = super::native_compile_options(&header_input, &observations)
        .expect("validate header inputs");

    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        &observations,
    )
    .expect("initial native compilation");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        &observations,
    )
    .expect("cached native compilation");
    fs::write(&input, "changed input").expect("change native input");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        &observations,
    )
    .expect("invalidated native compilation");

    fs::write(&header, "#define BRIDGE_VERSION 2\n").expect("change only header bytes");
    let changed = super::native_compile_options(&header_input, &observations)
        .expect("validate changed header inputs");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &changed,
        &observations,
    )
    .expect("header-only change invalidates native object");
    let outside = root.join("nested");
    fs::create_dir(&outside).expect("create separate include root");
    assert!(
        super::native_compile_options(
            &[super::NativeHeaderInput {
                include_directory: &outside,
                header_files: &header_files,
            }],
            &observations
        )
        .is_err()
    );
    fs::remove_file(&header).expect("remove declared header");
    assert!(super::native_compile_options(&header_input, &observations).is_err());

    assert_eq!(
        fs::read_to_string(&log)
            .expect("read compiler log")
            .lines()
            .count(),
        3,
        "unchanged inputs cache; source-only and header-only changes recompile"
    );
    assert!(
        observations
            .0
            .into_inner()
            .expect("observation lock")
            .iter()
            .any(|row| row
                == "phase=native-object state=cached operation=compile program module subject=example/module")
    );
    fs::remove_dir_all(root).expect("remove cache scenario root");
}

#[test]
#[ignore = "requires owned generated module C and the configured GCC SDK"]
fn actual_gcc_module_completes_with_bounded_load_motion_policy() {
    let source = std::path::PathBuf::from(
        std::env::var_os("GERBIL_MODULE_DIAGNOSTIC_SOURCE").expect("owned module source"),
    );
    let gsc = std::path::PathBuf::from(
        std::env::var_os("GERBIL_GSC").expect("configured Gambit compiler"),
    );
    assert!(super::super::process::configured_gambit_gcc(&gsc));
    let root = std::env::temp_dir().join(format!("native-actual-module-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let object = root.join("module.o");
    compile_program_module(
        &gsc,
        &source,
        &object,
        "owned/diagnostic-module",
        "-O2 -Dmain=mrr_grammar_gambit_main",
        &Observations(Mutex::new(Vec::new())),
    )
    .unwrap();
    assert!(object.is_file());
    assert!(!object.with_extension("i").exists());
    assert!(!object.with_extension("s").exists());
    std::fs::remove_dir_all(root).unwrap();
}
