// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use super::{NativeLinkLibrary, complete_sdk_dependency_search};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "gerbil-link-search-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    Fixture(path)
}

#[test]
fn receipt_locates_implicit_archives_once_without_changing_link_order() {
    let root = fixture();
    fs::write(root.0.join("libcrypto.a"), "target crypto").unwrap();
    let mut search = vec![root.0.join("old-cellar")];
    let libraries = vec![
        NativeLinkLibrary::new("crypto"),
        NativeLinkLibrary::new("crypto"),
        NativeLinkLibrary::new("m"),
        NativeLinkLibrary::new("dl"),
    ];
    let mut probes = Vec::new();
    complete_sdk_dependency_search(&mut search, &libraries, |package| {
        probes.push(package.to_owned());
        Ok(vec![root.0.clone(), root.0.clone()])
    })
    .unwrap();
    assert_eq!(probes, ["libcrypto"]);
    assert_eq!(search.len(), 2);
    assert_eq!(
        libraries
            .iter()
            .map(NativeLinkLibrary::as_str)
            .collect::<Vec<_>>(),
        ["crypto", "crypto", "m", "dl"]
    );
    complete_sdk_dependency_search(&mut search, &libraries, |_| panic!("already located")).unwrap();
}

#[test]
fn receipt_rejects_missing_static_archive_and_relative_package_paths() {
    let root = fixture();
    let libraries = vec![NativeLinkLibrary::new("ssl")];
    let mut search = Vec::new();
    assert!(
        complete_sdk_dependency_search(&mut search, &libraries, |_| Ok(vec![
            "relative".into(),
            root.0.clone()
        ]))
        .is_err()
    );
    assert_eq!(search, [root.0.clone()]);
    assert!(
        complete_sdk_dependency_search(&mut Vec::new(), &libraries, |_| Err(
            "unavailable target package".into()
        ))
        .is_err()
    );
}
