// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::Gerbil;

#[test]
fn reports_real_runtime_version() {
    let version = Gerbil::from_env()
        .version()
        .expect("a supported CI lane must provide gxi");
    assert!(
        version.contains("Gerbil"),
        "unexpected version: {version:?}"
    );
}

#[test]
fn projects_real_scalar_value() {
    let value = Gerbil::from_env()
        .eval_i64("(displayln (+ 40 2))")
        .expect("real gxi scalar projection must succeed");
    assert_eq!(value, 42);
}
