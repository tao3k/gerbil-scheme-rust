//! Private runtime module file identities prevent shared writable DLL state.
use crate::{VerifiedPackage, sha256_file};
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
pub(super) fn copy(package: &VerifiedPackage, destination: &Path) -> io::Result<Vec<String>> {
    private_directory(destination)?;
    let groups = package
        .manifest()
        .native_modules
        .values()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for (name, resource) in &package.manifest().files {
        if !package
            .manifest()
            .load_paths
            .iter()
            .any(|root| Path::new(name).starts_with(root))
            && !groups.contains(name.as_str())
        {
            continue;
        }
        let source = package.root().join(name);
        let output = destination.join(name);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        // Copy, never hard-link native modules: each VM needs distinct file identity.
        fs::copy(&source, &output)?;
        if sha256_file(&output)? != resource.sha256 {
            return Err(io::Error::other("runtime namespace changed during copy"));
        }
    }
    for (logical, physical) in &package.manifest().native_modules {
        let alias = destination.join(logical);
        if let Some(parent) = alias.parent() {
            fs::create_dir_all(parent)?;
        }
        // Share only within this VM namespace. Every owner has its own image inode.
        fs::hard_link(destination.join(physical), alias)?;
    }
    package
        .manifest()
        .load_paths
        .iter()
        .map(|name| {
            destination
                .join(name)
                .into_os_string()
                .into_string()
                .map_err(|_| io::Error::other("UTF8 namespace path"))
        })
        .collect()
}
pub(super) fn retire(root: &Path) -> io::Result<()> {
    // Called only after all native owners have completed known-clean shutdown.
    match fs::remove_dir_all(root) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

static LINKED_NAMESPACE: AtomicU64 = AtomicU64::new(0);
pub(super) fn linked(package: &VerifiedPackage) -> io::Result<(Vec<String>, Option<PathBuf>)> {
    if package.manifest().native_modules.is_empty() {
        return Ok((
            package
                .load_paths()
                .iter()
                .map(|p| {
                    p.to_str()
                        .map(str::to_owned)
                        .ok_or_else(|| io::Error::other("UTF8 load path"))
                })
                .collect::<io::Result<_>>()?,
            None,
        ));
    }
    let directory = std::env::temp_dir().join(format!(
        "gerbil-python-modules-{}-{}",
        std::process::id(),
        LINKED_NAMESPACE.fetch_add(1, Ordering::Relaxed)
    ));
    match copy(package, &directory) {
        Ok(paths) => Ok((paths, Some(directory))),
        Err(error) => {
            let _ = retire(&directory);
            Err(error)
        }
    }
}

fn private_directory(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
}
