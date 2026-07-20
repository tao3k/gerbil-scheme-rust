use std::path::PathBuf;

use gerbil_scheme::GerbilToolchain;

#[test]
fn maps_conventional_program_names_through_the_toolchain() {
    let toolchain = GerbilToolchain::new(
        PathBuf::from("/opt/gerbil/bin/gxi"),
        PathBuf::from("/opt/gerbil/bin/gxc"),
        PathBuf::from("/opt/gerbil/bin/gsc"),
    );

    assert_eq!(
        toolchain.program("gxi"),
        PathBuf::from("/opt/gerbil/bin/gxi")
    );
    assert_eq!(
        toolchain.program("gxc"),
        PathBuf::from("/opt/gerbil/bin/gxc")
    );
    assert_eq!(
        toolchain.program("gsc"),
        PathBuf::from("/opt/gerbil/bin/gsc")
    );
}

#[test]
fn leaves_paths_and_unknown_program_names_unchanged() {
    let toolchain = GerbilToolchain::new(
        PathBuf::from("/opt/gerbil/bin/gxi"),
        PathBuf::from("/opt/gerbil/bin/gxc"),
        PathBuf::from("/opt/gerbil/bin/gsc"),
    );

    assert_eq!(toolchain.program("gxpkg"), PathBuf::from("gxpkg"));
    assert_eq!(toolchain.program("./gxi"), PathBuf::from("./gxi"));
    assert_eq!(toolchain.program("bin/gxc"), PathBuf::from("bin/gxc"));
    assert_eq!(
        toolchain.program("/usr/local/bin/gsc"),
        PathBuf::from("/usr/local/bin/gsc"),
    );
}
