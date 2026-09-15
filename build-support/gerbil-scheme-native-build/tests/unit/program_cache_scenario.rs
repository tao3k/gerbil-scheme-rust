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
    let observations = Observations(Mutex::new(Vec::new()));

    compile_program_module(&compiler, &input, &output, "example/module", &observations)
        .expect("initial native compilation");
    compile_program_module(&compiler, &input, &output, "example/module", &observations)
        .expect("cached native compilation");
    fs::write(&input, "changed input").expect("change native input");
    compile_program_module(&compiler, &input, &output, "example/module", &observations)
        .expect("invalidated native compilation");

    assert_eq!(
        fs::read_to_string(&log)
            .expect("read compiler log")
            .lines()
            .count(),
        2,
        "the unchanged input must skip exactly one compiler invocation"
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
