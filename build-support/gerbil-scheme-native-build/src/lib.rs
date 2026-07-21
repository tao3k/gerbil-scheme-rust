//! Builds reusable native Gerbil/Gambit artifacts and the archive consumed by
//! `gerbil-scheme-sys`.

mod archive;
mod discovery;
mod generated_scm;
mod header;
mod native;
mod toolchain;

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
pub use native::build_native_archive;
pub use toolchain::{NativeCCompilerTool, discover_native_c_compiler};
