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

#[derive(Clone, Copy)]
pub(super) struct NativeObjectJob<'a> {
    pub source: &'a Path,
    pub object: &'a Path,
    pub compilation: ObjectCompilation<'a>,
    pub operation: ProgramArchiveOperation<'a>,
}

pub(super) fn compile_program_modules(
    gsc: &Path,
    sources: &[ProgramObject],
    cc_options: &str,
    additional_jobs: &[NativeObjectJob<'_>],
    observer: &dyn ProgramArchiveObserver,
) -> Result<(), String> {
    let metadata_options = metadata_compile_options(cc_options, true);
    let jobs: Vec<_> = additional_jobs
        .iter()
        .copied()
        .chain(sources.iter().map(|item| NativeObjectJob {
            source: &item.source,
            object: &item.object,
            compilation: ObjectCompilation {
                identity: "program-module-object-v1",
                options: if item.deferred {
                    &metadata_options
                } else {
                    cc_options
                },
                layout: item.layout,
            },
            operation: ProgramArchiveOperation {
                phase: "native-object",
                operation: "compile program module",
                subject: Some(&item.module),
            },
        }))
        .collect();
    let worker_count = native_build_parallelism(jobs.len());
    let batch = format!("jobs={} workers={worker_count}", jobs.len());
    observe_program_archive_operation(
        observer,
        ProgramArchiveOperation {
            phase: "native-object-batch",
            operation: "compile program native objects",
            subject: Some(&batch),
        },
        || {
            for_each_native_job(&jobs, worker_count, |item| {
                compile_native_object(
                    gsc,
                    item.source,
                    item.object,
                    item.compilation,
                    item.operation,
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

#[cfg(test)]
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

#[cfg(test)]
#[path = "../../tests/unit/program_object_pool.rs"]
mod object_pool_tests;
