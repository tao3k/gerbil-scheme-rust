// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Compiler-owned AOT archive contracts and native process execution.
mod archive;
mod metadata;
mod objects;
use metadata::{add_link_input, scheme_string};
#[cfg(test)]
use objects::compile_program_module;
use objects::compile_program_modules;
mod headers;
use headers::native_compile_options;
mod process;
pub(crate) use process::configured_gambit_gcc;
use process::{
    GSC_PROGRESS_OPTIONS, gambit_progress_command, gcc_allocation_diagnostics,
    native_progress_enabled, run_process, run_with_compiler_artifacts,
};

pub use archive::{
    NativeHeaderInput, ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramArchiveRequest, build_program_archive,
    build_program_archive_observed, build_program_archive_with_contract,
    observe_program_archive_operation, source_workspace,
};

pub use process::run_native_process;

pub(crate) use archive::{
    cached_native_output, compile_native_object, native_build_parallelism,
    native_input_fingerprint, publish_native_output_fingerprint,
};
