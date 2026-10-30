#[path = "../../src/generated_scm.rs"]
mod implementation;

use std::fs;

use implementation::{generated_scm_receipt, validate_generated_scm, workspace_input_fingerprint};

use super::support::unique_temp_dir;

const INPUT_FINGERPRINT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn generated_scm_input_fingerprint_tracks_all_build_inputs() {
    let workspace = unique_temp_dir("gerbil-native-build-generated-scm");
    fs::create_dir_all(workspace.join("scheme")).expect("create Scheme input root");
    fs::write(workspace.join("build.ss"), "build-v1\n").expect("write build.ss");
    fs::write(workspace.join("gerbil.pkg"), "package-v1\n").expect("write gerbil.pkg");
    fs::write(workspace.join("scheme/runtime.ss"), "native-v1\n").expect("write native.ss");
    fs::write(
        workspace.join("scheme/runtime-object-kinds.ss"),
        "object-kinds-v1\n",
    )
    .expect("write object kinds include");
    fs::write(
        workspace.join("scheme/runtime-collections.ss"),
        "collections-v1\n",
    )
    .expect("write runtime collections include");
    fs::write(
        workspace.join("scheme/runtime-exchange.ss"),
        "exchange-v1\n",
    )
    .expect("write runtime exchange include");
    fs::write(
        workspace.join("scheme/runtime-composites.ss"),
        "composites-v1\n",
    )
    .expect("write composite include");
    fs::write(workspace.join("scheme/utf8.ss"), "encoder-v1\n").expect("write utf8.ss");
    fs::write(workspace.join("scheme/runtime.ssi"), "ffi-v1\n").expect("write native.ssi");
    let original = workspace_input_fingerprint(&workspace);

    fs::write(workspace.join("scheme/runtime.ss"), "native-v2\n").expect("change native.ss");
    let changed = workspace_input_fingerprint(&workspace);

    assert_ne!(original, changed);
    fs::write(workspace.join("scheme/utf8.ss"), "encoder-v2\n").expect("change utf8.ss");
    assert_ne!(changed, workspace_input_fingerprint(&workspace));
    let before_include = workspace_input_fingerprint(&workspace);
    fs::write(
        workspace.join("scheme/runtime-collections.ss"),
        "collections-v2\n",
    )
    .expect("change runtime collections include");
    assert_ne!(before_include, workspace_input_fingerprint(&workspace));
    let before_composites = workspace_input_fingerprint(&workspace);
    fs::write(
        workspace.join("scheme/runtime-composites.ss"),
        "composites-v2\n",
    )
    .expect("change composite include");
    assert_ne!(before_composites, workspace_input_fingerprint(&workspace));
    let before_kinds = workspace_input_fingerprint(&workspace);
    fs::write(
        workspace.join("scheme/runtime-object-kinds.ss"),
        "object-kinds-v2\n",
    )
    .expect("change object kinds include");
    assert_ne!(before_kinds, workspace_input_fingerprint(&workspace));
    let before_exchange = workspace_input_fingerprint(&workspace);
    fs::write(
        workspace.join("scheme/runtime-exchange.ss"),
        "exchange-v2\n",
    )
    .expect("change runtime exchange include");
    assert_ne!(before_exchange, workspace_input_fingerprint(&workspace));
    fs::remove_dir_all(workspace).expect("remove Scheme input root");
}

#[test]
fn whole_file_receipt_rejects_any_file_or_input_change() {
    let contents = b";; compiler comment\n(define answer 42)\n";
    let receipt = generated_scm_receipt(contents, INPUT_FINGERPRINT);
    assert_eq!(
        validate_generated_scm(contents, &receipt, INPUT_FINGERPRINT),
        Ok(())
    );
    for changed in [
        b";; changed comment\n(define answer 42)\n".as_slice(),
        b";; compiler comment\n(define answer 43)\n".as_slice(),
    ] {
        assert!(validate_generated_scm(changed, &receipt, INPUT_FINGERPRINT).is_err());
    }
    assert!(validate_generated_scm(contents, &receipt, "changed-inputs").is_err());
    assert!(validate_generated_scm(contents, "", INPUT_FINGERPRINT).is_err());
}

#[test]
fn tracked_generated_scm_matches_current_workspace_inputs() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("build crate lives below the workspace build-support directory");
    let fingerprint = workspace_input_fingerprint(workspace);
    for module in ["runtime", "utf8"] {
        let scm = workspace.join(format!("scheme/generated/{module}.scm"));
        let contents = fs::read(&scm).expect("read tracked compiler output");
        let receipt = fs::read_to_string(scm.with_extension("scm.sha256"))
            .expect("read tracked compiler receipt");
        assert_eq!(
            validate_generated_scm(&contents, &receipt, &fingerprint),
            Ok(()),
            "{module}: regenerate compiler output with just bridge-regenerate"
        );
    }
}
