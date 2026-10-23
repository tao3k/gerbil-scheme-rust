// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_python::select_backend;

#[test]
fn explicit_selection_never_uses_the_other_provider() {
    assert_eq!(select_backend("scheme", Some(1), Some(2)), Ok(1));
    assert_eq!(select_backend("rust", Some(1), Some(2)), Ok(2));
    assert!(select_backend("rust", Some(1), None).is_err());
    assert!(select_backend("scheme", None, Some(2)).is_err());
    for name in ["", "auto", "Rust", "tokio", "scheme-or-rust"] {
        assert!(select_backend(name, Some(1), Some(2)).is_err());
    }
}
