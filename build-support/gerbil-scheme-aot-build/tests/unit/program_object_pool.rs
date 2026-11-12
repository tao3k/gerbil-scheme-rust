// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::{
    CFunctionLayout, NativeObjectJob, ObjectCompilation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramObject, compile_program_modules, for_each_native_job,
};
use crate::program::ProgramArchiveObservation;
use std::{
    path::Path,
    sync::{
        Barrier, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
struct Observations(Mutex<Vec<(String, String, String)>>);

impl ProgramArchiveObserver for Observations {
    fn observe(&self, event: ProgramArchiveObservation<'_>) {
        self.0.lock().unwrap().push((
            event.phase.into(),
            event.state.into(),
            event.subject.unwrap_or_default().into(),
        ));
    }
}

#[test]
fn module_startup_and_linker_complete_inside_one_native_batch() {
    let root = std::env::temp_dir().join(format!("gerbil-object-pool-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let compiler = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/unit/scenarios/program-content-cache/inputs/fake-gsc.sh");
    let module_c = root.join("module.c");
    let startup_c = root.join("startup.c");
    let linker_c = root.join("linker.c");
    for source in [&module_c, &startup_c, &linker_c] {
        std::fs::write(source, "/* immutable compiler input */").unwrap();
    }
    let startup_o = root.join("startup.o");
    let linker_o = root.join("linker.o");
    let jobs = [
        (&linker_c, &linker_o, "linker"),
        (&startup_c, &startup_o, "startup"),
    ]
    .map(|(source, object, subject)| NativeObjectJob {
        source,
        object,
        compilation: ObjectCompilation {
            identity: subject,
            options: "-O1",
            layout: CFunctionLayout::CompilerDefault,
        },
        operation: ProgramArchiveOperation {
            phase: "native-object",
            operation: subject,
            subject: Some(subject),
        },
    });
    let modules = [ProgramObject {
        source: module_c,
        object: root.join("module.o"),
        module: "module".into(),
        deferred: false,
        layout: CFunctionLayout::CompilerDefault,
    }];
    let observer = Observations::default();
    compile_program_modules(&compiler, &modules, "-O1", &jobs, &observer).unwrap();
    for object in [&modules[0].object, &startup_o, &linker_o] {
        assert!(object.is_file());
    }
    let events = observer.0.into_inner().unwrap();
    assert_eq!(events.first().unwrap().0, "native-object-batch");
    assert!(events.first().unwrap().2.starts_with("jobs=3 workers="));
    assert_eq!(events.last().unwrap().0, "native-object-batch");
    assert_eq!(events.last().unwrap().1, "complete");
    for subject in ["module", "startup", "linker"] {
        assert_eq!(
            events
                .iter()
                .filter(|(phase, state, name)| phase == "native-object"
                    && state == "complete"
                    && name == subject)
                .count(),
            1
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failing_native_job_joins_all_running_workers_within_the_same_capacity() {
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let barrier = Barrier::new(4);
    let result = for_each_native_job(&[0, 1, 2, 3, 4, 5, 6, 7], 4, |index| {
        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(current, Ordering::SeqCst);
        if *index < 4 {
            barrier.wait();
        }
        active.fetch_sub(1, Ordering::SeqCst);
        if *index == 0 {
            Err("injected linker failure".into())
        } else {
            Ok(())
        }
    });
    assert_eq!(result, Err("injected linker failure".into()));
    assert_eq!(peak.load(Ordering::SeqCst), 4);
    assert_eq!(active.load(Ordering::SeqCst), 0);
}
