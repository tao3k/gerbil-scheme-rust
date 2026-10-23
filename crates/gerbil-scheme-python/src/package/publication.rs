// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Checked generated-root replacement; failures retain staging and backups.
use super::inventory::{ensure_directories, invalid};
use super::{PackageManifest, VerifiedPackage};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) fn publish(
    package: &VerifiedPackage,
    destination: &Path,
) -> io::Result<VerifiedPackage> {
    package.reverify()?;
    fs::create_dir_all(destination)?;
    let destination = destination.canonicalize()?;
    let marker = destination.join(".gerbil-resources.json");
    let previous = read_previous(&marker)?;
    check_ownership(package.manifest(), previous.as_ref(), &destination)?;
    if previous.as_ref() == Some(package.manifest()) {
        return package.manifest().verify(&destination);
    }
    publish_changed(package, previous.as_ref(), &destination, &marker)
}

fn read_previous(marker: &Path) -> io::Result<Option<PackageManifest>> {
    match fs::symlink_metadata(marker) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            PackageManifest::from_json(&fs::read(marker)?).map(Some)
        }
        Ok(_) => Err(invalid("aliased publication manifest")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn check_ownership(
    next: &PackageManifest,
    previous: Option<&PackageManifest>,
    destination: &Path,
) -> io::Result<()> {
    if let Some(previous) = previous {
        previous.verify(destination)?;
    }
    let old_roots: BTreeSet<&Path> = previous
        .into_iter()
        .flat_map(|manifest| manifest.roots.iter().map(Path::new))
        .collect();
    for name in &next.roots {
        let root = Path::new(name);
        reject_root_overlap(root, &old_roots)?;
        if !old_roots.contains(root) {
            require_absent(&destination.join(root))?;
        }
        if let Some(parent) = root.parent() {
            ensure_directories(destination, parent)?;
        }
    }
    Ok(())
}

fn reject_root_overlap(root: &Path, old_roots: &BTreeSet<&Path>) -> io::Result<()> {
    let has_ancestor = root
        .ancestors()
        .skip(1)
        .any(|parent| old_roots.contains(parent));
    let first_descendant = old_roots
        .range::<Path, _>((std::ops::Bound::Included(root), std::ops::Bound::Unbounded))
        .next();
    let has_descendant = first_descendant.is_some_and(|old| root != *old && old.starts_with(root));
    if has_ancestor || has_descendant {
        return Err(invalid("ambiguous generated root ownership transition"));
    }
    Ok(())
}

fn require_absent(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(invalid("unowned generated package output")),
    }
}

fn publish_changed(
    package: &VerifiedPackage,
    previous: Option<&PackageManifest>,
    destination: &Path,
    marker: &Path,
) -> io::Result<VerifiedPackage> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stage = destination.join(format!(
        ".gerbil-stage-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&stage)?;
    let incoming = stage.join("incoming");
    package.copy_to(&incoming)?;
    let backup = stage.join("previous");
    fs::create_dir(&backup)?;
    if let Some(previous) = previous {
        move_roots(&previous.roots, destination, &backup)?;
    }
    move_roots(&package.manifest().roots, &incoming, destination)?;
    let verified = package.manifest().verify(destination)?;
    let temporary_marker = stage.join("manifest.json");
    fs::write(&temporary_marker, serde_json::to_vec(package.manifest())?)?;
    fs::rename(temporary_marker, marker)?;
    // Delete only the verified old inventory after the new manifest is published.
    fs::remove_dir_all(stage)?;
    Ok(verified)
}

fn move_roots(roots: &[String], source: &Path, destination: &Path) -> io::Result<()> {
    for name in roots {
        let root = Path::new(name);
        if let Some(parent) = root.parent() {
            ensure_directories(destination, parent)?;
        }
        fs::rename(source.join(root), destination.join(root))?;
    }
    Ok(())
}
