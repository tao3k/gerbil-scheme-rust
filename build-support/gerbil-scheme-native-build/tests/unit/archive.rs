use gerbil_scheme_native_build::{build_static_archive_from_link_plan, discover_native_c_compiler};
use std::{fs, path::Path, process::Command};

use gerbil_scheme_native_build::{
    CargoDirectiveKind, NativeLinkLibrary, NativeStaticLinkPlan, static_archive_cargo_directives,
    static_archive_file_name,
};
use std::path::PathBuf;

#[test]
fn static_archive_projection_is_parameterized_and_ordered() {
    let plan = NativeStaticLinkPlan {
        module_objects: vec![PathBuf::from("module.o")],
        link_object: PathBuf::from("link.o"),
        link_search_dirs: vec![PathBuf::from("/opt/gambit/lib")],
        link_libraries: vec![NativeLinkLibrary::new("gambit")],
    };
    let directives = static_archive_cargo_directives(
        "consumer_native",
        PathBuf::from("/tmp/archive").as_path(),
        &plan,
    );
    assert_eq!(directives[0].kind, CargoDirectiveKind::RustcLinkSearch);
    assert_eq!(
        directives[1].line(),
        "cargo:rustc-link-lib=static=consumer_native"
    );
    assert_eq!(
        directives[2].line(),
        "cargo:rustc-link-search=native=/opt/gambit/lib"
    );
    assert_eq!(directives[3].line(), "cargo:rustc-link-lib=gambit");
}

#[test]
fn static_archive_filename_matches_target_family() {
    let file_name = static_archive_file_name("consumer_native");
    if cfg!(target_env = "msvc") {
        assert_eq!(file_name, "consumer_native.lib");
    } else {
        assert_eq!(file_name, "libconsumer_native.a");
    }
}

#[test]
#[cfg(unix)]
fn static_archive_packaging_consumes_real_object_files() {
    let Ok(compiler) = discover_native_c_compiler() else {
        return;
    };
    let root = super::support::unique_temp_dir("gerbil-native-build-archive");
    let objects = root.join("objects");
    let archive_dir = root.join("archive");
    fs::create_dir_all(&objects).expect("create object directory");
    fs::write(
        objects.join("module.c"),
        "int module_value(void) { return 1; }\n",
    )
    .expect("write module source");
    fs::write(
        objects.join("link.c"),
        "int link_value(void) { return 2; }\n",
    )
    .expect("write link source");
    let module_object = objects.join("module.o");
    let link_object = objects.join("link.o");
    compile_c_object(&compiler.program, &objects.join("module.c"), &module_object);
    compile_c_object(&compiler.program, &objects.join("link.c"), &link_object);

    let plan = NativeStaticLinkPlan {
        module_objects: vec![module_object],
        link_object,
        link_search_dirs: Vec::new(),
        link_libraries: Vec::new(),
    };
    let receipt = build_static_archive_from_link_plan("consumer_native", &plan, &archive_dir)
        .expect("package real object files");
    assert!(receipt.archive_file.is_file());
    assert_eq!(receipt.archive_name, "consumer_native");
    fs::remove_dir_all(root).expect("remove archive root");
}

#[cfg(unix)]
fn compile_c_object(compiler: &Path, source: &Path, object: &Path) {
    let status = Command::new(compiler)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(object)
        .status()
        .expect("run C compiler");
    assert!(
        status.success(),
        "C compiler failed for {}",
        source.display()
    );
    assert!(object.is_file(), "missing object {}", object.display());
}
