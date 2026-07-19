use gerbil_scheme_native_build::discover_gambit_link_search_dir_from_gsc;
use std::fs;

use super::support::unique_temp_dir;

#[test]
fn discovers_gambit_library_from_a_gerbil_wrapper() {
    let prefix = unique_temp_dir("gerbil-native-build-prefix");
    let wrapper_root = unique_temp_dir("gerbil-native-build-wrapper");
    let wrapper_bin = wrapper_root.join("bin");
    let real_bin = prefix.join("bin");
    let lib = prefix.join("lib");
    fs::create_dir_all(&wrapper_bin).expect("create wrapper bin");
    fs::create_dir_all(&real_bin).expect("create real bin");
    fs::create_dir_all(&lib).expect("create library directory");
    fs::write(real_bin.join("gsc"), "").expect("write fake gsc");
    fs::write(lib.join("libgambit.a"), "").expect("write fake Gambit library");
    fs::write(
        wrapper_bin.join("gsc"),
        format!(
            "#!/bin/sh\nexport GERBIL_HOME=\"{}\"\nexec \"{}\" \"$@\"\n",
            prefix.display(),
            real_bin.join("gsc").display()
        ),
    )
    .expect("write wrapper");

    let discovery = discover_gambit_link_search_dir_from_gsc(&wrapper_bin.join("gsc"))
        .expect("discover Gambit from wrapper");
    assert_eq!(discovery.search_dir, lib);
    assert!(discovery.library_path.ends_with("libgambit.a"));
    fs::remove_dir_all(prefix).expect("remove prefix");
    fs::remove_dir_all(wrapper_root).expect("remove wrapper root");
}
