// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_python::{
    ModuleClosure, ModuleRoot, PackageManifest, ResourceTree, stage_module_closure,
    stage_resources, stage_source_inventory,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "gerbil-python-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("source")).unwrap();
        fs::write(root.join("source/code.ssi"), b"compiled metadata").unwrap();
        fs::write(root.join("source/code.o1"), b"compiled library").unwrap();
        Self(root)
    }
    fn stage(&self) -> PackageManifest {
        stage_resources(
            &self.0.join("package"),
            &[ResourceTree::complete(
                &self.0.join("source"),
                "_runtime/library",
            )],
            &["_runtime/library".into()],
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn source_inventory_publishes_verified_files_without_build_tools() {
    let fixture = Fixture::new();
    let output = fixture.0.join("output");
    fs::create_dir(&output).unwrap();
    let package = fixture.0.join("installed");
    let manifest_path = stage_source_inventory(
        &output,
        &package,
        &[ResourceTree::file(
            &fixture.0.join("source/code.ssi"),
            "_sources/code.ssi",
        )],
        "_sources",
    )
    .unwrap();
    let manifest: PackageManifest =
        serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.roots, ["_sources"]);
    manifest.verify(&package).unwrap();
    assert!(output.join("source_inventory.json").is_file());
    fs::write(package.join("_sources/code.ssi"), b"changed").unwrap();
    assert!(manifest.verify(&package).is_err());
    assert!(
        stage_source_inventory(
            &output,
            &package,
            &[ResourceTree::file(
                &fixture.0.join("source/code.ssi"),
                "_sources/code.ssi"
            )],
            "_sources"
        )
        .is_err()
    );
}

#[test]
fn individual_inputs_have_complete_owned_parent_inventories() {
    let fixture = Fixture::new();
    let target = fixture.0.join("individual");
    let manifest = stage_resources(
        &target,
        &[
            ResourceTree::file(&fixture.0.join("source/code.ssi"), "sources/meta/code.ssi"),
            ResourceTree::file(&fixture.0.join("source/code.o1"), "sources/meta/code.o1"),
        ],
        &[],
    )
    .unwrap();
    assert_eq!(manifest.roots, ["sources/meta"]);
    assert_eq!(manifest.files.len(), 2);
    manifest.verify(&target).unwrap();
    fs::write(target.join("sources/meta/unexpected"), b"not declared").unwrap();
    assert!(manifest.verify(&target).is_err());
}

#[test]
fn generated_publication_removes_only_verified_obsolete_roots() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let installed = fixture.0.join("published");
    manifest
        .verify(&fixture.0.join("package"))
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    fs::write(installed.join("__init__.py"), b"user source").unwrap();
    let next_root = fixture.0.join("next");
    let next = stage_resources(
        &next_root,
        &[ResourceTree::complete(&fixture.0.join("source"), "sources")],
        &[],
    )
    .unwrap();
    next.verify(&next_root)
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    assert!(!installed.join("_runtime/library").exists());
    assert_eq!(
        fs::read(installed.join("__init__.py")).unwrap(),
        b"user source"
    );
    next.verify(&installed).unwrap();
}

#[test]
fn root_transition_rejects_unowned_collision_without_removing_old_roots() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let installed = fixture.0.join("published");
    manifest
        .verify(&fixture.0.join("package"))
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    fs::create_dir(installed.join("sources")).unwrap();
    fs::write(installed.join("sources/user.txt"), b"not generated").unwrap();
    let next_root = fixture.0.join("next");
    let next = stage_resources(
        &next_root,
        &[ResourceTree::complete(&fixture.0.join("source"), "sources")],
        &[],
    )
    .unwrap();
    assert!(
        next.verify(&next_root)
            .unwrap()
            .publish_to(&installed)
            .is_err()
    );
    manifest.verify(&installed).unwrap();
    assert_eq!(
        fs::read(installed.join("sources/user.txt")).unwrap(),
        b"not generated"
    );
}

#[test]
fn root_transition_rejects_changed_previous_inventory() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let installed = fixture.0.join("published");
    manifest
        .verify(&fixture.0.join("package"))
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    let next_root = fixture.0.join("next");
    let next = stage_resources(
        &next_root,
        &[ResourceTree::complete(&fixture.0.join("source"), "sources")],
        &[],
    )
    .unwrap();
    fs::write(installed.join("_runtime/library/code.o1"), b"changed").unwrap();
    assert!(
        next.verify(&next_root)
            .unwrap()
            .publish_to(&installed)
            .is_err()
    );
    assert_eq!(
        fs::read(installed.join("_runtime/library/code.o1")).unwrap(),
        b"changed"
    );
    assert!(!installed.join("sources").exists());
}

#[test]
fn root_transition_rejects_ancestor_ownership_changes() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let installed = fixture.0.join("published");
    manifest
        .verify(&fixture.0.join("package"))
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    let next_root = fixture.0.join("next");
    let next = stage_resources(
        &next_root,
        &[ResourceTree::complete(
            &fixture.0.join("source"),
            "_runtime",
        )],
        &[],
    )
    .unwrap();
    assert!(
        next.verify(&next_root)
            .unwrap()
            .publish_to(&installed)
            .is_err()
    );
    manifest.verify(&installed).unwrap();
}

#[test]
fn manifest_ancestor_membership_is_component_based() {
    let fixture = Fixture::new();
    let mut manifest = fixture.stage();
    manifest.roots.push("_runtime/library/nested".into());
    assert!(PackageManifest::from_json(&serde_json::to_vec(&manifest).unwrap()).is_err());
    manifest.roots.pop();
    manifest.load_paths = vec!["_runtime/library-sibling".into()];
    assert!(PackageManifest::from_json(&serde_json::to_vec(&manifest).unwrap()).is_err());
}

#[test]
fn relocatable_complete_inventory_does_not_consult_origin() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let manifest = PackageManifest::from_json(&bytes).unwrap();
    assert_eq!(manifest.files.len(), 2);
    fs::rename(fixture.0.join("package"), fixture.0.join("moved")).unwrap();
    fs::remove_dir_all(fixture.0.join("source")).unwrap();
    let package = manifest.verify(&fixture.0.join("moved")).unwrap();
    assert_eq!(
        package.load_paths(),
        vec![package.root().join("_runtime/library")]
    );
    package.reverify().unwrap();
}
#[test]
fn changed_same_length_resource_is_rejected() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    fs::write(
        fixture.0.join("package/_runtime/library/code.o1"),
        b"corrupted binary",
    )
    .unwrap();
    assert!(manifest.verify(&fixture.0.join("package")).is_err());
}

#[test]
fn duplicate_json_resources_are_rejected() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let resource = serde_json::to_string(manifest.files.values().next().unwrap()).unwrap();
    let json = format!(
        r#"{{"schema":"gerbil-scheme-python.package.v1","roots":["_runtime"],"load_paths":[],"files":{{"_runtime/file":{resource},"_runtime/file":{resource}}}}}"#
    );
    assert!(PackageManifest::from_json(json.as_bytes()).is_err());
}

#[test]
fn generated_publication_updates_only_verified_owned_roots() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = manifest.verify(&fixture.0.join("package")).unwrap();
    let installed = fixture.0.join("published");
    fs::create_dir(&installed).unwrap();
    fs::write(installed.join("__init__.py"), b"user source").unwrap();
    package.publish_to(&installed).unwrap();
    fs::write(fixture.0.join("source/code.o1"), b"new compiled artifact").unwrap();
    let next_root = fixture.0.join("next");
    let next = stage_resources(
        &next_root,
        &[ResourceTree::complete(
            &fixture.0.join("source"),
            "_runtime/library",
        )],
        &["_runtime/library".into()],
    )
    .unwrap();
    next.verify(&next_root)
        .unwrap()
        .publish_to(&installed)
        .unwrap();
    assert_eq!(
        fs::read(installed.join("__init__.py")).unwrap(),
        b"user source"
    );
    assert_eq!(
        fs::read(installed.join("_runtime/library/code.o1")).unwrap(),
        b"new compiled artifact"
    );
    assert!(package.publish_to(&installed).is_ok());
    manifest.verify(&installed).unwrap();
}

#[test]
fn generated_publication_rejects_unowned_or_modified_output() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = manifest.verify(&fixture.0.join("package")).unwrap();
    let installed = fixture.0.join("unowned");
    fs::create_dir_all(installed.join("_runtime/library")).unwrap();
    assert!(package.publish_to(&installed).is_err());
    let owned = fixture.0.join("owned");
    package.publish_to(&owned).unwrap();
    fs::write(
        owned.join("_runtime/library/code.o1"),
        b"modified by someone else",
    )
    .unwrap();
    assert!(package.publish_to(&owned).is_err());
}

#[test]
fn official_closure_rejects_missing_duplicate_and_foreign_inputs() {
    let fixture = Fixture::new();
    let source = fixture.0.join("source").canonicalize().unwrap();
    let mut closure = ModuleClosure {
        schema: "gerbil-scheme-python.closure.v1".into(),
        entries: vec![":example".into()],
        files: vec![source.join("code.o1"), source.join("missing.ssi")],
    };
    let roots = [ModuleRoot {
        source: &source,
        destination: "_runtime",
    }];
    assert!(stage_module_closure(&fixture.0.join("bad"), &closure, &roots, &[]).is_err());
    assert!(!fixture.0.join("bad").exists());
    closure.files = vec![source.join("code.o1"), source.join("code.o1")];
    assert!(stage_module_closure(&fixture.0.join("bad"), &closure, &roots, &[]).is_err());
    fs::write(fixture.0.join("foreign.o1"), b"outside root").unwrap();
    closure.files = vec![fixture.0.join("foreign.o1").canonicalize().unwrap()];
    assert!(stage_module_closure(&fixture.0.join("bad"), &closure, &roots, &[]).is_err());
}

#[test]
fn copy_preserves_conflicting_candidate_and_rechecks_origin() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = manifest.verify(&fixture.0.join("package")).unwrap();
    let target = fixture.0.join("installed");
    package.copy_to(&target).unwrap();
    let object = target.join("_runtime/library/code.o1");
    fs::write(&object, b"different candidate").unwrap();
    assert!(package.copy_to(&target).is_err());
    assert_eq!(fs::read(object).unwrap(), b"different candidate");
}

#[cfg(unix)]
#[test]
fn copy_rejects_broken_alias_without_writing_outside_package() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = manifest.verify(&fixture.0.join("package")).unwrap();
    let target = fixture.0.join("installed");
    fs::create_dir_all(target.join("_runtime/library")).unwrap();
    let outside = fixture.0.join("outside");
    std::os::unix::fs::symlink(&outside, target.join("_runtime/library/code.o1")).unwrap();
    assert!(package.copy_to(&target).is_err());
    assert!(!outside.exists());
}
#[test]
fn missing_and_extra_files_are_not_hidden() {
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = fixture.0.join("package");
    fs::write(package.join("_runtime/library/extra"), b"extra").unwrap();
    assert!(manifest.verify(&package).is_err());
    fs::remove_file(package.join("_runtime/library/extra")).unwrap();
    fs::remove_file(package.join("_runtime/library/code.ssi")).unwrap();
    assert!(manifest.verify(&package).is_err());
}
#[test]
fn unsafe_paths_are_rejected_before_staging() {
    let fixture = Fixture::new();
    for name in [
        "../escape",
        "/absolute",
        "a/./b",
        "a//b",
        "a/../b",
        "a\\b",
        "",
        "a/",
    ] {
        assert!(
            stage_resources(
                &fixture.0.join("package"),
                &[ResourceTree::complete(&fixture.0.join("source"), name)],
                &[name.into()]
            )
            .is_err(),
            "{name}"
        );
        assert!(!fixture.0.join("package").exists());
    }
}
#[test]
fn overlapping_and_duplicate_roots_are_rejected() {
    let fixture = Fixture::new();
    for other in ["_runtime", "_runtime/nested"] {
        assert!(
            stage_resources(
                &fixture.0.join("package"),
                &[
                    ResourceTree::complete(&fixture.0.join("source"), "_runtime"),
                    ResourceTree::complete(&fixture.0.join("source"), other)
                ],
                &[]
            )
            .is_err()
        );
    }
}
#[test]
fn existing_destination_is_not_overwritten() {
    let fixture = Fixture::new();
    fixture.stage();
    let marker = fixture.0.join("package/owned.txt");
    fs::write(&marker, b"keep").unwrap();
    assert!(
        stage_resources(
            &fixture.0.join("package"),
            &[ResourceTree::complete(
                &fixture.0.join("source"),
                "_runtime"
            )],
            &[]
        )
        .is_err()
    );
    assert_eq!(fs::read(marker).unwrap(), b"keep");
}
#[cfg(unix)]
#[test]
fn symlinked_files_and_parent_directories_are_rejected() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let manifest = fixture.stage();
    let package = fixture.0.join("package");
    let object = package.join("_runtime/library/code.o1");
    fs::remove_file(&object).unwrap();
    symlink(fixture.0.join("source/code.o1"), &object).unwrap();
    assert!(manifest.verify(&package).is_err());
    fs::remove_file(object).unwrap();
    fs::copy(
        fixture.0.join("source/code.o1"),
        package.join("_runtime/library/code.o1"),
    )
    .unwrap();
    fs::rename(package.join("_runtime"), package.join("original")).unwrap();
    symlink("original", package.join("_runtime")).unwrap();
    assert!(manifest.verify(&package).is_err());
}
#[cfg(unix)]
#[test]
fn staging_does_not_follow_source_symlinks() {
    let fixture = Fixture::new();
    std::os::unix::fs::symlink("code.o1", fixture.0.join("source/alias")).unwrap();
    assert!(
        stage_resources(
            &fixture.0.join("package"),
            &[ResourceTree::complete(
                &fixture.0.join("source"),
                "_runtime"
            )],
            &[]
        )
        .is_err()
    );
}
#[test]
fn schema_and_digest_validation_are_closed() {
    let fixture = Fixture::new();
    let mut manifest = fixture.stage();
    manifest.schema = "unknown".into();
    assert!(manifest.verify(Path::new("/does-not-exist")).is_err());
    manifest.schema = "gerbil-scheme-python.package.v1".into();
    manifest.files.values_mut().next().unwrap().sha256 = "G".repeat(64);
    assert!(PackageManifest::from_json(&serde_json::to_vec(&manifest).unwrap()).is_err());
}

#[test]
fn loader_inventory_excludes_debug_symbols_not_loadable_modules() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("source/build.scm"), b"compiler intermediate").unwrap();
    fs::create_dir_all(
        fixture
            .0
            .join("source/code.o1.dSYM/Contents/Resources/DWARF"),
    )
    .unwrap();
    fs::write(
        fixture
            .0
            .join("source/code.o1.dSYM/Contents/Resources/DWARF/code.o1"),
        b"debugger only",
    )
    .unwrap();
    let source = fixture.0.join("source").canonicalize().unwrap();
    let closure = ModuleClosure {
        schema: "gerbil-scheme-python.closure.v1".into(),
        entries: vec![":example/library".into()],
        files: vec![source.join("code.o1"), source.join("code.ssi")],
    };
    let manifest = stage_module_closure(
        &fixture.0.join("package"),
        &closure,
        &[ModuleRoot {
            source: &source,
            destination: "_runtime",
        }],
        &[],
    )
    .unwrap();
    assert_eq!(
        manifest
            .files
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["_runtime/code.o1", "_runtime/code.ssi"]
    );
    assert!(fixture.0.join("source/build.scm").exists());
    assert!(
        fixture
            .0
            .join("source/code.o1.dSYM/Contents/Resources/DWARF/code.o1")
            .exists()
    );
}
