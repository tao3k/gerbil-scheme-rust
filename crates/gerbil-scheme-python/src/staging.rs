// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Build-side resource staging. No compiler invocation or Scheme source parsing.
use crate::{PackageManifest, PackageResource, sha256_file};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Complete library-owned input tree and its package-relative destination.
#[derive(Clone, Copy, Debug)]
pub struct ResourceTree<'a> {
    pub(crate) source: &'a Path,
    pub(crate) destination: &'a str,
    kind: ResourceKind,
}
#[derive(Clone, Copy, Debug)]
enum ResourceKind {
    Directory,
    File,
}
impl<'a> ResourceTree<'a> {
    /// Preserve every regular file, suitable for source evidence directories.
    #[must_use]
    pub const fn complete(source: &'a Path, destination: &'a str) -> Self {
        Self {
            source,
            destination,
            kind: ResourceKind::Directory,
        }
    }
    /// Preserve one regular input file, including its independent SHA identity.
    #[must_use]
    pub const fn file(source: &'a Path, destination: &'a str) -> Self {
        Self {
            source,
            destination,
            kind: ResourceKind::File,
        }
    }
}

/// Stage complete trees into a fresh package directory. The returned manifest
/// must be compiled into the extension; a wheel-provided manifest is not its
/// own trust anchor. Existing destinations are never replaced or cleaned.
///
/// # Errors
/// Preserves failed partial staging for diagnosis. Rejects aliases, overlapping
/// roots, non-regular inputs and source changes observed during copying.
pub fn stage_resources(
    destination: &Path,
    trees: &[ResourceTree<'_>],
    load_paths: &[String],
) -> io::Result<PackageManifest> {
    let declared = PackageManifest {
        schema: "gerbil-scheme-python.package.v1".into(),
        roots: trees
            .iter()
            .map(|tree| tree.destination.to_owned())
            .collect(),
        load_paths: vec![],
        files: BTreeMap::new(),
    };
    PackageManifest::from_json(&serde_json::to_vec(&declared)?)?;
    let mut manifest = PackageManifest {
        schema: "gerbil-scheme-python.package.v1".into(),
        roots: inventory_roots(trees)?,
        load_paths: load_paths.to_owned(),
        files: BTreeMap::new(),
    };
    // Validate all names before joining them to a destination or writing bytes.
    PackageManifest::from_json(&serde_json::to_vec(&manifest)?)?;
    fs::create_dir(destination)?;
    let destination = destination.canonicalize()?;
    for tree in trees {
        let source = tree.source.canonicalize()?;
        if fs::symlink_metadata(tree.source)?.file_type().is_symlink()
            || !match tree.kind {
                ResourceKind::Directory => source.is_dir(),
                ResourceKind::File => source.is_file(),
            }
            || destination.starts_with(&source)
            || source.starts_with(&destination)
        {
            return Err(io::Error::other("aliased or overlapping resource tree"));
        }
        stage_tree(
            &source,
            &destination,
            &PathBuf::from(tree.destination),
            &mut manifest.files,
        )?;
    }
    manifest.verify(&destination)?;
    Ok(manifest)
}
fn inventory_roots(trees: &[ResourceTree<'_>]) -> io::Result<Vec<String>> {
    let mut roots = std::collections::BTreeSet::new();
    for tree in trees {
        let path = Path::new(tree.destination);
        let root = match tree.kind {
            ResourceKind::Directory => path,
            ResourceKind::File => path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| {
                    io::Error::other("file resources require an owned parent directory")
                })?,
        };
        roots.insert(root.to_string_lossy().into_owned());
    }
    Ok(roots
        .iter()
        .filter(|name| {
            !Path::new(name)
                .ancestors()
                .skip(1)
                .any(|p| roots.contains(p.to_string_lossy().as_ref()))
        })
        .cloned()
        .collect())
}

/// Freeze declared inputs into one owned package root and embed its manifest.
/// Failed staging remains untouched; replacement uses verified publication only.
/// # Errors
/// Rejects missing/aliased inputs, incomplete root coverage and unowned output.
pub fn stage_source_inventory(
    output: &Path,
    package: &Path,
    trees: &[ResourceTree<'_>],
    root: &str,
) -> io::Result<PathBuf> {
    fs::create_dir_all(output)?;
    let stage = output.join(format!("source-inputs-{}", std::process::id()));
    let mut manifest = stage_resources(&stage, trees, &[])?;
    manifest.roots = vec![root.to_owned()];
    manifest.verify(&stage)?.publish_to(package)?;
    let manifest_path = output.join("source_inventory.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    manifest.verify(&stage)?;
    fs::remove_dir_all(&stage)?;
    Ok(manifest_path)
}
pub(crate) fn stage_tree(
    source: &Path,
    destination: &Path,
    relative: &Path,
    files: &mut BTreeMap<String, PackageResource>,
) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    let target = destination.join(relative);
    if metadata.is_dir() {
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            stage_tree(
                &entry.path(),
                destination,
                &relative.join(entry.file_name()),
                files,
            )?;
        }
    } else if metadata.is_file() {
        let sha256 = sha256_file(source)?;
        fs::create_dir_all(
            target
                .parent()
                .ok_or_else(|| io::Error::other("resource parent"))?,
        )?;
        fs::copy(source, &target)?;
        if sha256_file(&target)? != sha256 || sha256_file(source)? != sha256 {
            return Err(io::Error::other("source changed while staging package"));
        }
        let name = relative
            .to_str()
            .ok_or_else(|| io::Error::other("non-UTF8 resource path"))?;
        files.insert(
            name.into(),
            PackageResource {
                sha256,
                origin: source.to_string_lossy().into_owned(),
            },
        );
    } else {
        return Err(io::Error::other("resource symlink or special file"));
    }
    Ok(())
}
