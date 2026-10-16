#[test]
fn inventory_belongs_to_the_compiled_dependency() {
    let root = super::build_source_root().expect("dependency source root");
    assert_eq!(
        root.join("crates/gerbil-scheme-python").canonicalize().unwrap(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).canonicalize().unwrap(),
    );
    for relative in ["Cargo.toml", "Cargo.lock", "include/gerbil_scheme_owned.h",
        "build-support/gerbil-scheme-rust-ir/src/lib.rs"] {
        assert!(root.join(relative).is_file(), "missing {relative}");
    }
}
