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
fn indexed_source_match_preserves_order_boundaries_and_cursor_behavior() {
    let algorithms = [true, false]
        .into_iter()
        .map(|unicode_alphanumeric| {
            json!({
                "schema": SOURCE_MATCH_IR_SCHEMA,
                "name": if unicode_alphanumeric { "match_unicode" } else { "match_extra_only" },
                "scan": "utf8_character_boundaries",
                "candidate": "exact_target_prefix",
                "boundary": {
                    "unicode_alphanumeric": unicode_alphanumeric,
                    "extra_word_characters": "_-"
                },
                "winner": "longest_then_first"
            })
        })
        .map(|algorithm| compile_source_match_json(&algorithm.to_string()).expect("typed matcher"))
        .collect::<Vec<_>>()
        .join("\n");
    compile_and_run(
        &algorithms,
        &quote! {
            fn reference(
                source: &str,
                cursor: usize,
                targets: &[String],
                unicode_alphanumeric: bool,
            ) -> Option<(usize, usize, usize)> {
                let remaining = source.get(cursor..)?;
                let word = |ch: char| {
                    (unicode_alphanumeric && ch.is_alphanumeric()) || "_-".contains(ch)
                };
                for (relative, _) in remaining.char_indices() {
                    let start = cursor + relative;
                    if source.get(..start)?.chars().next_back().is_some_and(word) {
                        continue;
                    }
                    let tail = source.get(start..)?;
                    let mut best = None;
                    for (index, target) in targets.iter().enumerate() {
                        if target.is_empty() || !tail.starts_with(target) {
                            continue;
                        }
                        let end = start + target.len();
                        if source.get(end..)?.chars().next().is_some_and(word) {
                            continue;
                        }
                        if best.is_none_or(|(best_end, _)| end > best_end) {
                            best = Some((end, index));
                        }
                    }
                    if let Some((end, index)) = best {
                        return Some((start, end, index));
                    }
                }
                None
            }

            let mut targets = vec![
                "", "Alpha", "Alpha Beta", "Alpha", "é", "élan", "β", "ß",
                "A", "_", "-", "😀", "😀x", "target", "Target",
            ].into_iter().map(str::to_owned).collect::<Vec<_>>();
            for index in 0..32 {
                targets.push(format!("unmatched-{index}"));
            }
            for source in [
                "", "Alpha Beta Alpha Alphabet", "é élan β ß", "_Alpha-Alpha",
                "😀x 😀", "Aéβ", "target Target", "z\nAlpha",
            ] {
                for count in [0, 1, 3, 15, 17, 32, 33, targets.len()] {
                    let candidates = &targets[..count];
                    for cursor in 0..=source.len() + 1 {
                        assert_eq!(
                            match_unicode(source, cursor, candidates),
                            reference(source, cursor, candidates, true),
                            "unicode boundary: {source:?} at {cursor}, {count} targets"
                        );
                        assert_eq!(
                            match_extra_only(source, cursor, candidates),
                            reference(source, cursor, candidates, false),
                            "extra-only boundary: {source:?} at {cursor}, {count} targets"
                        );
                    }
                }
            }
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
