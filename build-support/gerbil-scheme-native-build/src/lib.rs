//! Builds reusable native Gerbil/Gambit artifacts and the archive consumed by
//! `gerbil-scheme-sys`.

mod archive;
mod discovery;
mod gambit_program;
mod generated_scm;
pub use gambit_program::{
    default_gambit_gsc_program_for_gxi, discover_gambit_gsc_from_env, is_gambit_gsc_program,
    resolve_gerbil_executable,
};
mod header;
mod native;
mod package_launcher;
mod program;
mod toolchain;
pub use program::{
    NativeHeaderInput, ProgramArchiveContract, ProgramArchiveObservation, ProgramArchiveObserver,
    ProgramArchiveOperation, ProgramArchiveRequest, build_program_archive,
    build_program_archive_observed, build_program_archive_with_contract,
    observe_program_archive_operation, source_workspace,
};

pub use archive::{
    CargoDirective, CargoDirectiveKind, NativeArchiveLinkReceipt, NativeLinkLibrary,
    NativeStaticLinkPlan, build_static_archive_from_link_plan, static_archive_cargo_directives,
    static_archive_file_name,
};
pub use discovery::{GambitLinkSearchDiscovery, discover_gambit_link_search_dir_from_gsc};
pub use header::{
    NativeCHeaderDriftReceipt, NativeCHeaderGenerationReceipt, validate_native_c_header,
    write_native_c_header,
};
pub use native::{build_native_archive, gerbil_command};
pub use package_launcher::prepare_gsc_progress_launcher;
pub use toolchain::{NativeCCompilerTool, discover_native_c_compiler};

pub use program::run_native_process;

#[path = "../../../crates/gerbil-scheme/src/native_environment.rs"]
mod native_environment;
pub use native_environment::{GerbilNativeToolEnvironment, configure_gerbil_native_tool_command};
