use gerbil_scheme_aot_build::discover_native_c_compiler;

#[test]
fn native_compiler_discovery_uses_the_cc_contract() {
    if let Ok(tool) = discover_native_c_compiler() {
        assert!(!tool.program.as_os_str().is_empty());
    }
}

#[cfg(unix)]
#[test]
fn paired_wrapper_compiler_rejects_ghostscript_and_follows_symlinked_home() {
    use gerbil_scheme_aot_build::{default_gambit_gsc_program_for_gxi, is_gambit_gsc_program};
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };
    let root = super::support::unique_temp_dir("gerbil-gsc-pair");
    let bin = root.join("bin");
    let home = root.join("versioned-runtime");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(home.join("bin")).unwrap();
    let ghost = bin.join("gsc");
    let gambit = home.join("bin/gsc");
    fs::write(&ghost, "#!/bin/sh\necho 'GPL Ghostscript 10.07.0'\n").unwrap();
    fs::write(&gambit, "#!/bin/sh\necho 'v4.9.5'\n").unwrap();
    for path in [&ghost, &gambit] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let current = root.join("current");
    symlink(&home, &current).unwrap();
    let gxi = bin.join("gxi");
    fs::write(
        &gxi,
        format!("#!/bin/sh\nexport GERBIL_HOME=\"{}\"\n", current.display()),
    )
    .unwrap();
    let linked = root.join("linked-gxi");
    symlink(&gxi, &linked).unwrap();
    assert!(!is_gambit_gsc_program(&ghost));
    assert_eq!(
        default_gambit_gsc_program_for_gxi(&linked),
        current.join("bin/gsc")
    );
    fs::write(
        &gambit,
        "#!/bin/sh\necho 'dcd677c 20260908015653 aarch64-apple-darwin25.6.0 \"./configure'\n",
    )
    .unwrap();
    assert!(is_gambit_gsc_program(&gambit));
    fs::remove_dir_all(root).unwrap();
}
