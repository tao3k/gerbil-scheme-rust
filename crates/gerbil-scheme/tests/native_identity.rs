// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

#![cfg(feature = "native")]

use gerbil_scheme::GerbilRuntime;

#[test]
fn identity_i64_crosses_the_live_gerbil_abi() {
    let runtime = GerbilRuntime::initialize().expect("initialize native Gerbil runtime");

    assert_eq!(runtime.identity_i64(0).expect("identity zero"), 0);
    assert_eq!(
        runtime
            .identity_i64(i64::MIN)
            .expect("identity lower bound"),
        i64::MIN
    );
    assert_eq!(
        runtime
            .identity_i64(i64::MAX)
            .expect("identity upper bound"),
        i64::MAX
    );
}
