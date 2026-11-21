// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Parallel directory work retains the original per-file verification rules.
use super::inventory::{PackageResource, inspect};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

pub(super) fn children(
    root: &Path,
    path: &Path,
    expected: &BTreeMap<String, PackageResource>,
    seen: &mut BTreeSet<String>,
) -> io::Result<()> {
    let children = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    let results = children
        .par_iter()
        .map(|child| {
            let mut local = BTreeSet::new();
            inspect(root, child, expected, &mut local)?;
            Ok(local)
        })
        .collect::<Vec<io::Result<BTreeSet<String>>>>();
    for result in results {
        seen.extend(result?);
    }
    Ok(())
}
