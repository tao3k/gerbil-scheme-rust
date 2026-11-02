// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use super::{NativeLinkLibrary, validate_native_dependency_search};
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
fn native_receipt_is_validated_without_changing_link_order() {
    let root = fixture();
    fs::write(root.0.join("libcrypto.a"), "target crypto").unwrap();
    let search = vec![root.0.join("old-cellar"), root.0.clone()];
    let libraries = vec![
        NativeLinkLibrary::new("crypto"),
        NativeLinkLibrary::new("crypto"),
        NativeLinkLibrary::new("m"),
        NativeLinkLibrary::new("dl"),
    ];
    validate_native_dependency_search(&search, &libraries).unwrap();
    assert_eq!(search, [root.0.join("old-cellar"), root.0.clone()]);
    assert_eq!(
        libraries
            .iter()
            .map(NativeLinkLibrary::as_str)
            .collect::<Vec<_>>(),
        ["crypto", "crypto", "m", "dl"]
    );
}

#[test]
fn missing_native_archive_is_rejected_without_a_host_package_fallback() {
    let root = fixture();
    let libraries = vec![NativeLinkLibrary::new("ssl")];
    assert!(validate_native_dependency_search(std::slice::from_ref(&root.0), &libraries).is_err());
    fs::write(root.0.join("libcrypto.a"), "other library").unwrap();
    assert!(validate_native_dependency_search(std::slice::from_ref(&root.0), &libraries).is_err());
    fs::write(root.0.join("libssl.a"), "target ssl").unwrap();
    validate_native_dependency_search(std::slice::from_ref(&root.0), &libraries).unwrap();
}

#[test]
fn dynamic_dependency_is_not_reinterpreted_as_a_static_archive() {
    let root = fixture();
    fs::write(root.0.join("libcrypto.dylib"), "target crypto").unwrap();
    let search = [root.0.clone()];
    validate_native_dependency_search(&search, &[NativeLinkLibrary::new("crypto")]).unwrap();
    assert!(
        validate_native_dependency_search(&search, &[NativeLinkLibrary::new("static=crypto")])
            .is_err()
    );
}
