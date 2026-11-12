// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

#[test]
fn lazy_metadata_keeps_headers_and_the_configured_compiler_default() {
    for level in ["0", "1", "2", "3", "s", "z"] {
        let options = format!("-O{level} -I'/package headers' -DHEADER_DIGEST=1");
        assert_eq!(
            super::metadata_compile_options(&options, true),
            " -I'/package headers' -DHEADER_DIGEST=1"
        );
        assert_eq!(super::metadata_compile_options(&options, false), options);
        assert_eq!(
            super::metadata_compile_options(&format!("-O{level}"), true),
            ""
        );
    }
}
