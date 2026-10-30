// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Transport the official expander's exact dependency inventory, not an SDK scan.
use crate::{PackageManifest, ResourceTree};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

/// Build-owned output of `gerbil-rs-stage-load-closure`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleClosure {
    pub schema: String,
    pub entries: Vec<String>,
    pub files: Vec<PathBuf>,
}
/// One compiler-selected module root and its installed package location.
#[derive(Clone, Copy, Debug)]
pub struct ModuleRoot<'a> {
    pub source: &'a Path,
    pub destination: &'a str,
}

/// Copy only the expander-resolved runtime/phase closure, plus explicit complete
/// source-evidence trees. No recursive discovery is performed under module roots.
/// # Errors
/// Rejects unknown schemas, duplicate/aliased files, undeclared roots, source
/// changes, missing resources and ambiguous root mappings. Partial output stays
/// available for diagnosis; it is never returned as a verified package.
pub fn stage_module_closure(
    destination: &Path,
    closure: &ModuleClosure,
    roots: &[ModuleRoot<'_>],
    sources: &[ResourceTree<'_>],
) -> io::Result<PackageManifest> {
    let mut manifest = closure_manifest(closure, roots, sources)?;
    let selected = select_modules(closure, roots)?;
    stage_selected(destination, &mut manifest, selected, sources)?;
    manifest.verify(destination)?;
    Ok(manifest)
}

fn closure_manifest(
    closure: &ModuleClosure,
    roots: &[ModuleRoot<'_>],
    sources: &[ResourceTree<'_>],
) -> io::Result<PackageManifest> {
    if closure.schema != "gerbil-scheme-python.closure.v1"
        || closure.entries.is_empty()
        || closure.files.is_empty()
    {
        return Err(io::Error::other("invalid or empty official module closure"));
    }
    let manifest = PackageManifest {
        schema: "gerbil-scheme-python.package.v1".into(),
        roots: roots
            .iter()
            .map(|r| r.destination.to_owned())
            .chain(crate::staging::inventory_roots(sources)?)
            .collect(),
        load_paths: roots.iter().map(|r| r.destination.to_owned()).collect(),
        files: BTreeMap::new(),
        native_modules: BTreeMap::new(),
    };
    PackageManifest::from_json(&serde_json::to_vec(&manifest)?)
}

fn select_modules<'a>(
    closure: &'a ModuleClosure,
    roots: &[ModuleRoot<'_>],
) -> io::Result<BTreeMap<&'a PathBuf, PathBuf>> {
    let native_roots = roots
        .iter()
        .map(|r| r.source.canonicalize())
        .collect::<io::Result<Vec<_>>>()?;
    let mut selected = BTreeMap::new();
    for path in &closure.files {
        if !path.is_absolute()
            || path.canonicalize()? != *path
            || !fs::symlink_metadata(path)?.is_file()
        {
            return Err(io::Error::other(
                "module closure contains an aliased or nonregular file",
            ));
        }
        let mappings = native_roots
            .iter()
            .enumerate()
            .filter_map(|(i, r)| path.strip_prefix(r).ok().map(|p| (i, p)))
            .collect::<Vec<_>>();
        if mappings.len() != 1 {
            return Err(io::Error::other(
                "module source must belong to exactly one declared root",
            ));
        }
        let (index, relative) = mappings[0];
        let target = Path::new(roots[index].destination).join(relative);
        if selected.insert(path, target).is_some() {
            return Err(io::Error::other("duplicate module closure file"));
        }
    }
    Ok(selected)
}

fn stage_selected(
    destination: &Path,
    manifest: &mut PackageManifest,
    selected: BTreeMap<&PathBuf, PathBuf>,
    sources: &[ResourceTree<'_>],
) -> io::Result<()> {
    fs::create_dir(destination)?;
    for root in &manifest.roots {
        fs::create_dir_all(destination.join(root))?;
    }
    for (source, relative) in selected {
        if let Some(parent) = relative.parent() {
            fs::create_dir_all(destination.join(parent))?;
        }
        crate::staging::stage_tree(source, destination, &relative, &mut manifest.files)?;
    }
    for tree in sources {
        crate::staging::stage_tree(
            &tree.source.canonicalize()?,
            destination,
            Path::new(tree.destination),
            &mut manifest.files,
        )?;
    }
    Ok(())
}

/// Compiler-selected grouping and its verified publication owner.
#[derive(Debug)]
pub struct ModuleBundleRequest<'a> {
    pub root: &'a Path,
    pub manifest: &'a mut PackageManifest,
    pub closure: &'a ModuleClosure,
    pub roots: &'a [ModuleRoot<'a>],
    pub groups: &'a [(String, Vec<PathBuf>)],
}

/// Replace compiler-selected native parts with physical groups already in the
/// staged inventory. Source metadata and every logical loader path are retained.
/// # Errors
/// Rejects unknown sources, undeclared targets, collisions and invalid manifests.
pub fn coalesce_module_resources(request: ModuleBundleRequest<'_>) -> io::Result<()> {
    let ModuleBundleRequest {
        root,
        manifest,
        closure,
        roots,
        groups,
    } = request;
    let selected = select_modules(closure, roots)?;
    for (target, sources) in groups {
        coalesce_group(root, manifest, &selected, target, sources)?;
    }
    manifest.verify(root)?;
    Ok(())
}

fn coalesce_group(
    root: &Path,
    manifest: &mut PackageManifest,
    selected: &BTreeMap<&PathBuf, PathBuf>,
    target: &str,
    sources: &[PathBuf],
) -> io::Result<()> {
    if !manifest.files.contains_key(target) {
        return Err(io::Error::other("undeclared native group"));
    }
    for source in sources {
        let logical = selected
            .get(source)
            .ok_or_else(|| io::Error::other("native part outside compiler closure"))?;
        let name = logical
            .to_str()
            .ok_or_else(|| io::Error::other("UTF8 logical module"))?
            .to_owned();
        if manifest.files.remove(&name).is_none()
            || manifest
                .native_modules
                .insert(name, target.to_owned())
                .is_some()
        {
            return Err(io::Error::other("duplicate native module grouping"));
        }
        fs::remove_file(root.join(logical))?;
    }
    Ok(())
}
