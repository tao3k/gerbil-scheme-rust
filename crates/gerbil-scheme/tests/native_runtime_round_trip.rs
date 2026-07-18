// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use gerbil_scheme::{GERBIL_SCHEME_RUST_ABI_VERSION, GerbilRuntime};

#[test]
fn initialized_runtime_crosses_the_live_gerbil_abi() {
    let runtime = GerbilRuntime::initialize().expect("initialize the live Gerbil runtime");

    assert_eq!(
        runtime.abi_version().expect("query the live ABI version"),
        GERBIL_SCHEME_RUST_ABI_VERSION,
    );
    assert_eq!(
        runtime
            .add_i64(20, 22)
            .expect("call the exported Gerbil procedure"),
        42,
    );
}
