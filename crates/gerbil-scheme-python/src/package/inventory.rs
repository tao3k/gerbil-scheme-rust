// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Verify complete resource inventories and publish only byte-verified owned roots.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

/// One exact resource; origin is diagnostic provenance, not a runtime path.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PackageResource {
    pub sha256: String,
    pub origin: String,
}

/// Library-owned, versioned resource contract, embedded in the extension.
/// The complete declared directories are checked, including unexpected files.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub schema: String,
    pub roots: Vec<String>,
    pub load_paths: Vec<String>,
    #[serde(deserialize_with = "unique_files")]
    pub files: BTreeMap<String, PackageResource>,
}

fn unique_files<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, PackageResource>, D::Error> {
    struct Files;
    impl<'de> serde::de::Visitor<'de> for Files {
        type Value = BTreeMap<String, PackageResource>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique package resource paths")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut files = BTreeMap::new();
            while let Some((name, value)) = map.next_entry::<String, PackageResource>()? {
                if files.insert(name, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate resource path"));
                }
            }
            Ok(files)
        }
    }
    deserializer.deserialize_map(Files)
}

/// Complete SHA-checked package admission. This is not a sandbox or a claim of
/// immunity to concurrent host mutations; reverify at an explicit trust boundary.
#[derive(Clone, Debug)]
pub struct VerifiedPackage {
    root: PathBuf,
    manifest: PackageManifest,
}

pub(super) fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}
fn relative(name: &str) -> io::Result<&Path> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains('\\')
        || name.contains('\0')
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.to_str() != Some(name)
        || name.split('/').any(str::is_empty)
        || name.split('/').any(|c| c == "." || c == "..")
    {
        return Err(invalid(format!("non-canonical package path: {name}")));
    }
    Ok(path)
}

/// Stream a full file without loading the resource inventory into memory.
/// # Errors
/// Propagates file-open and read errors.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 65536].into_boxed_slice();
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        digest.update(&buffer[..length]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

impl PackageManifest {
    /// Read a manifest from trusted build bytes, not from an unverified wheel file.
    /// # Errors
    /// Rejects unknown fields, schema versions, aliases and ambiguous inventories.
    pub fn from_json(bytes: &[u8]) -> io::Result<Self> {
        let value: Self = serde_json::from_slice(bytes).map_err(invalid)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema != "gerbil-scheme-python.package.v1" || self.roots.is_empty() {
            return Err(invalid("unsupported or empty package contract"));
        }
        let mut roots = BTreeSet::new();
        for name in &self.roots {
            let root = relative(name)?;
            if !roots.insert(root) {
                return Err(invalid("duplicate package root"));
            }
        }
        for a in &roots {
            if a.ancestors().skip(1).any(|parent| roots.contains(parent)) {
                return Err(invalid("overlapping package roots"));
            }
        }
        let mut loads = BTreeSet::new();
        for name in &self.load_paths {
            let path = relative(name)?;
            if !loads.insert(path) || !path.ancestors().any(|parent| roots.contains(parent)) {
                return Err(invalid("undeclared or duplicate load path"));
            }
        }
        for (name, resource) in &self.files {
            let path = relative(name)?;
            if !path
                .ancestors()
                .skip(1)
                .any(|parent| roots.contains(parent))
                || resource.sha256.len() != 64
                || !resource
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid(format!("invalid resource entry: {name}")));
            }
        }
        Ok(())
    }

    /// Admit installed package bytes without SDK variables, tool discovery or compilation.
    /// # Errors
    /// Rejects missing/extra resources, changed bytes, symlinks and special files.
    pub fn verify(&self, root: &Path) -> io::Result<VerifiedPackage> {
        self.validate()?;
        let root = root.canonicalize()?;
        let mut seen = BTreeSet::new();
        for name in &self.roots {
            inspect(&root, &root.join(name), &self.files, &mut seen)?;
        }
        if seen.len() != self.files.len() {
            return Err(invalid("incomplete package inventory"));
        }
        for name in &self.load_paths {
            if !root.join(name).is_dir() {
                return Err(invalid("load path is not a directory"));
            }
        }
        Ok(VerifiedPackage {
            root,
            manifest: self.clone(),
        })
    }
}

fn inspect(
    root: &Path,
    path: &Path,
    expected: &BTreeMap<String, PackageResource>,
    seen: &mut BTreeSet<String>,
) -> io::Result<()> {
    // Check each component before descent; canonicalize alone hides directory symlinks.
    let kind = fs::symlink_metadata(path)?.file_type();
    if path.canonicalize()? != path {
        return Err(invalid(format!(
            "non-canonical resource: {}",
            path.display()
        )));
    }
    if kind.is_dir() {
        for entry in fs::read_dir(path)? {
            inspect(root, &entry?.path(), expected, seen)?;
        }
    } else if kind.is_file() {
        let name = path
            .strip_prefix(root)
            .map_err(invalid)?
            .to_str()
            .ok_or_else(|| invalid("non-UTF8 package path"))?;
        let resource = expected
            .get(name)
            .ok_or_else(|| invalid(format!("undeclared resource: {name}")))?;
        if path.canonicalize()? != path || sha256_file(path)? != resource.sha256 {
            return Err(invalid(format!("resource identity mismatch: {name}")));
        }
        seen.insert(name.to_owned());
    } else {
        return Err(invalid(format!(
            "resource alias or special file: {}",
            path.display()
        )));
    }
    Ok(())
}

impl VerifiedPackage {
    /// Publish build-owned generated roots, preserving caller Python sources.
    /// A checked sidecar proves ownership of a previous generated inventory;
    /// it is never the runtime trust anchor (that manifest is compiled in).
    /// # Errors
    /// Rejects unowned/changed previous output. Failed partial publication is
    /// retained and cannot satisfy either compiled manifest.
    pub fn publish_to(&self, destination: &Path) -> io::Result<Self> {
        super::publication::publish(self, destination)
    }
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
    #[must_use]
    pub fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }
    /// Package-relative official loader roots, in declared precedence order.
    #[must_use]
    pub fn load_paths(&self) -> Vec<PathBuf> {
        self.manifest
            .load_paths
            .iter()
            .map(|p| self.root.join(p))
            .collect()
    }
    /// Revalidate all bytes, without a metadata-only cache shortcut.
    /// # Errors
    /// The same errors as initial admission.
    pub fn reverify(&self) -> io::Result<Self> {
        self.manifest.verify(&self.root)
    }
    /// Copy a verified build-owned inventory into a Python package. Existing
    /// bytes must already match; this never overwrites a different candidate.
    /// # Errors
    /// Rejects aliases, conflicting files, undeclared resources and copy drift.
    pub fn copy_to(&self, destination: &Path) -> io::Result<Self> {
        self.reverify()?;
        fs::create_dir_all(destination)?;
        let destination = destination.canonicalize()?;
        for root in &self.manifest.roots {
            ensure_directories(&destination, Path::new(root))?;
        }
        for (name, resource) in &self.manifest.files {
            let target = destination.join(name);
            if let Some(parent) = target.parent() {
                ensure_directories(
                    &destination,
                    parent.strip_prefix(&destination).map_err(invalid)?,
                )?;
            }
            match fs::symlink_metadata(&target) {
                Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                    return Err(invalid("aliased destination resource"));
                }
                Ok(_) => {
                    if sha256_file(&target)? != resource.sha256 {
                        return Err(invalid(format!("different package candidate at {name}")));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    fs::copy(self.root.join(name), &target)?;
                }
                Err(error) => return Err(error),
            }
        }
        self.manifest.verify(&destination)
    }
}

pub(super) fn ensure_directories(root: &Path, relative: &Path) -> io::Result<()> {
    let mut directory = root.to_owned();
    for part in relative.components() {
        directory.push(part);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(invalid("aliased package directory")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&directory)?,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
