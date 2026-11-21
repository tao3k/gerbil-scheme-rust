// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

#[test]
fn cargo_optimization_levels_keep_the_standalone_default() {
    assert_eq!(super::optimization_option(None).unwrap(), "-O2");
    assert_eq!(super::optimization_option(Some("0")).unwrap(), "-O1");
    for level in ["1", "2", "3", "s", "z"] {
        assert_eq!(
            super::optimization_option(Some(level)).unwrap(),
            format!("-O{level}")
        );
    }
    assert!(super::optimization_option(Some("1; exit 0")).is_err());
}
