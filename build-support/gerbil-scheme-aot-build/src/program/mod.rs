// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Compiler-owned AOT archive contracts and native process execution.
mod archive;
mod headers;
mod process;
pub(crate) use process::configured_gambit_gcc;

pub use archive::{
    NativeHeaderInput, ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramArchiveRequest, build_program_archive,
    build_program_archive_observed, build_program_archive_with_contract,
    observe_program_archive_operation, source_workspace,
};

pub use process::run_native_process;
