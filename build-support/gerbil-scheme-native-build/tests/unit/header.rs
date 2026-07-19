use gerbil_scheme_native_build::validate_native_c_header;
use std::fs;

use super::support::unique_temp_dir;

#[test]
fn header_drift_receipt_fails_closed_without_selecting_a_generator() {
    let root = unique_temp_dir("gerbil-native-build-header");
    fs::create_dir_all(&root).expect("create header root");
    let expected = root.join("expected.h");
    let actual = root.join("actual.h");
    fs::write(&expected, "int stable(void);\n").expect("write expected header");
    fs::write(&actual, "int drifted(void);\n").expect("write actual header");
    let receipt = validate_native_c_header(&expected, &actual).expect("compare headers");
    assert!(!receipt.matched);
    fs::remove_dir_all(root).expect("remove header root");
}
