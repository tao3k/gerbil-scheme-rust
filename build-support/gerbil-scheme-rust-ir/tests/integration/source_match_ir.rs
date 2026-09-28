use gerbil_scheme_rust_ir::{SOURCE_MATCH_IR_SCHEMA, compile_ir_json, compile_source_match_json};
use quote::quote;
use serde_json::json;

use super::event_ir::compile_and_run;

#[test]
fn scheme_source_match_ir_typechecks_and_matches_longest_at_unicode_boundaries() {
    let algorithm = json!({
        "schema": SOURCE_MATCH_IR_SCHEMA,
        "name": "next_source_match",
        "scan": "utf8_character_boundaries",
        "candidate": "exact_target_prefix",
        "boundary": {
            "unicode_alphanumeric": true,
            "extra_word_characters": "_-"
        },
        "winner": "longest_then_first"
    });
    let source = compile_source_match_json(&algorithm.to_string()).expect("typed matcher IR");
    assert_eq!(
        source,
        compile_ir_json(&algorithm.to_string()).expect("IR dispatch")
    );
    compile_and_run(
        &source,
        &quote! {
            let targets = vec!["Alpha".to_owned(), "Alpha Beta".to_owned()];
            assert_eq!(next_source_match("Alpha Beta Alpha Alphabet", 0, &targets),
                       Some((0, 10, 1)));
            assert_eq!(next_source_match("Alpha Beta Alpha Alphabet", 10, &targets),
                       Some((11, 16, 0)));
            assert_eq!(next_source_match("é Alpha", 0, &targets),
                       Some((3, 8, 0)));
            assert_eq!(next_source_match("Alpha-Beta", 0, &targets), None);
            assert_eq!(next_source_match("Alpha", 1, &targets), None);
        },
    );
}

#[test]
fn source_match_ir_rejects_unknown_algorithm_fields() {
    let algorithm = json!({
        "schema": SOURCE_MATCH_IR_SCHEMA,
        "name": "next_source_match",
        "scan": "utf8_character_boundaries",
        "candidate": "exact_target_prefix",
        "boundary": {"unicode_alphanumeric": true, "extra_word_characters": "_-"},
        "winner": "longest_then_first",
        "rust_source": "panic!()"
    });
    assert!(compile_source_match_json(&algorithm.to_string()).is_err());
}
