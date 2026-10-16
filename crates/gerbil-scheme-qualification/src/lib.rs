//! Producer-owned full-program AOT qualification, isolated from the default
//! scalar archive. Benchmark workload exports are not production bridge APIs.

/// The compiler-owned full Gerbil graph used only by actor qualification.
#[cfg(feature = "actor-aot")]
#[must_use]
pub fn linked_program() -> gerbil_scheme::LinkedGerbilProgram {
    unsafe extern "C" {
        fn ___LNK_gerbil__actor__qualification(
            state: *mut gerbil_scheme_sys::GerbilGlobalState,
        ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    }
    // SAFETY: build.rs admits exactly one ABI-v1 bridge module and the complete
    // compiler-owned program graph. Its main returns without running a loop.
    unsafe { gerbil_scheme::LinkedGerbilProgram::from_linker(___LNK_gerbil__actor__qualification) }
}
