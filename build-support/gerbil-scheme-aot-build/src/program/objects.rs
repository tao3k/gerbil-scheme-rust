// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned bounded native compilation, including SDK metadata policy.
use super::{
    ProgramArchiveObserver, ProgramArchiveOperation, compile_native_object,
    native_build_parallelism, observe_program_archive_operation,
};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

pub(super) fn compile_program_modules(
    gsc: &Path,
    sources: &[(PathBuf, PathBuf, String, bool)],
    cc_options: &str,
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
        || compile_program_modules_inner(gsc, sources, cc_options, observer, worker_count),
    )
}

fn compile_program_modules_inner(
    gsc: &Path,
    sources: &[(PathBuf, PathBuf, String, bool)],
    cc_options: &str,
    observer: &dyn ProgramArchiveObserver,
    worker_count: usize,
) -> Result<(), String> {
    if worker_count <= 1 {
        for (source, object, module, deferred) in sources {
            let options = metadata_compile_options(cc_options, *deferred);
            compile_program_module(gsc, source, object, module, &options, observer)?;
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
                    let Some((source, object, module, deferred)) = sources.get(index) else {
                        return;
                    };
                    let options = metadata_compile_options(cc_options, *deferred);
                    if let Err(error) =
                        compile_program_module(gsc, source, object, module, &options, observer)
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

fn metadata_compile_options(options: &str, deferred: bool) -> String {
    // Gerbil gsc-cc-options(phi? = #t) omits runtime optimization flags.
    // Retain declared package headers and let the frozen SDK choose its default.
    if deferred {
        options.replacen("-O2", "", 1)
    } else {
        options.to_owned()
    }
}

pub(super) fn compile_program_module(
    gsc: &Path,
    source: &Path,
    object: &Path,
    module: &str,
    cc_options: &str,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    compile_native_object(
        gsc,
        source,
        object,
        "program-module-object-v1",
        cc_options,
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile program module",
            subject: Some(module),
        },
        observer,
    )
}
