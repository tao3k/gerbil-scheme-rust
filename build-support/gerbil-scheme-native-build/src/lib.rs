//! Builds the native Gerbil and Gambit archive consumed by `gerbil-scheme-sys`.

mod native;

pub use native::build_native_archive;
