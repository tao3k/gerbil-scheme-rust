use std::path::Path;

use gerbil_scheme::{Gerbil, GerbilError, GerbilToolchain};

#[test]
fn explicit_toolchain_preserves_paths() {
    let toolchain = GerbilToolchain::new("/x/gxi", "/x/gxc", "/x/gsc");
    assert_eq!(toolchain.gxi(), Path::new("/x/gxi"));
    assert_eq!(toolchain.gxc(), Path::new("/x/gxc"));
    assert_eq!(toolchain.gsc(), Path::new("/x/gsc"));
}

#[test]
fn missing_interpreter_is_reported_at_spawn_boundary() {
    let runtime = Gerbil::new(GerbilToolchain::new(
        "/definitely/missing/gxi",
        "gxc",
        "gsc",
    ));
    assert!(matches!(runtime.version(), Err(GerbilError::Spawn { .. })));
}
