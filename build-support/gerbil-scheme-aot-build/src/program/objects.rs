// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned bounded native compilation, including SDK metadata policy.
use super::{
    CFunctionLayout, ObjectCompilation, ProgramArchiveObserver, ProgramArchiveOperation,
    compile_native_object, native_build_parallelism, observe_program_archive_operation,
};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

pub(super) struct ProgramObject {
    pub source: PathBuf,
    pub object: PathBuf,
    pub module: String,
    pub deferred: bool,
    pub layout: CFunctionLayout,
}

pub(super) fn compile_program_modules(
    gsc: &Path,
    sources: &[ProgramObject],
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
        || {
            for_each_native_job(sources, worker_count, |item| {
                let options = metadata_compile_options(cc_options, item.deferred);
                compile_program_module(
                    gsc,
                    &item.source,
                    &item.object,
                    &item.module,
                    &options,
                    item.layout,
                    observer,
                )
            })
        },
    )
}

pub(super) fn for_each_native_job<T: Sync>(
    jobs: &[T],
    worker_count: usize,
    run: impl Fn(&T) -> Result<(), String> + Sync,
) -> Result<(), String> {
    if worker_count <= 1 {
        for job in jobs {
            run(job)?;
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
                    let Some(job) = jobs.get(index) else {
                        return;
                    };
                    if let Err(error) = run(job) {
                        failure
                            .lock()
                            .expect("native build failure lock")
                            .get_or_insert(error);
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
        let prefix_end = options.find(' ').unwrap_or(options.len());
        options[prefix_end..].to_owned()
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
    layout: CFunctionLayout,
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    compile_native_object(
        gsc,
        source,
        object,
        ObjectCompilation {
            identity: "program-module-object-v1",
            options: cc_options,
            layout,
        },
        ProgramArchiveOperation {
            phase: "native-object",
            operation: "compile program module",
            subject: Some(module),
        },
        observer,
    )
}

#[cfg(test)]
#[path = "../../tests/unit/program_metadata_profile.rs"]
mod metadata_profile_tests;
