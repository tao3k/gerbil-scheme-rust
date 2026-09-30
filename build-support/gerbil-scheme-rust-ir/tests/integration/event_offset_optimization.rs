//! Semantics and code-size checks for bounded event offset lowering.

use super::event_ir::{compile_and_run, stateful_document};
use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

#[test]
fn consecutive_line_steps_lower_to_one_bounded_offset() {
    let mut document = stateful_document();
    let step = |from| json!({"kind": "line_step", "from": from});
    document["line"] = json!([{
        "kind": "token",
        "syntax_kind": 1,
        "start": "start",
        "end": step(step(step(json!("start"))))
    }]);
    document["finish"] = json!([]);
    let source =
        compile_event_function_json(&document.to_string()).expect("consecutive line steps compile");
    assert!(source.contains("saturating_add(3"));
    assert_eq!(source.matches("saturating_add(").count(), 1);
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("abcd\n"), vec![
                StartNode(0), Token { kind: 1, start: 0, end: 3 }, FinishNode,
            ]);
            assert_eq!(parse_events("a"), vec![
                StartNode(0), Token { kind: 1, start: 0, end: 1 }, FinishNode,
            ]);
        },
    );
}

#[test]
fn horizontal_skip_is_shared_without_changing_offsets() {
    let mut document = stateful_document();
    let skip = json!({"kind": "line_skip_horizontal", "from": "start"});
    document["line"] = json!([{
        "kind": "token",
        "syntax_kind": 1,
        "start": skip,
        "end": {"kind": "line_step", "from": skip}
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("source-backed horizontal skip compiles");
    assert_eq!(source.matches("fn __event_skip_horizontal(").count(), 1);
    assert_eq!(
        source
            .matches("__event_skip_horizontal(bytes, end,")
            .count(),
        2
    );
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events(" \tA\n"), vec![
                StartNode(0), Token { kind: 1, start: 2, end: 3 }, FinishNode,
            ]);
            assert_eq!(parse_events("\t"), vec![
                StartNode(0), FinishNode,
            ]);
        },
    );
}

#[test]
fn line_end_offsets_share_helpers_without_changing_source_ranges() {
    let mut document = stateful_document();
    let content_end = json!({"kind": "line_content_end"});
    let trimmed_end = json!({"kind": "line_trim_end"});
    let trimmed_from = json!({"kind": "line_trim_end_from", "from": {
        "kind": "line_step", "from": "start"
    }});
    document["line"] = json!([
        {"kind": "token", "syntax_kind": 1, "start": "start", "end": content_end},
        {"kind": "token", "syntax_kind": 2, "start": "start", "end": trimmed_end},
        {"kind": "token", "syntax_kind": 3, "start": "start", "end": trimmed_from}
    ]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("source-backed line ends compile");
    assert_eq!(source.matches("fn __event_line_content_end(").count(), 1);
    assert_eq!(source.matches("fn __event_trim_whitespace_end(").count(), 1);
    assert_eq!(source.matches("__event_trim_whitespace_end(").count(), 3);
    assert!(source.contains("#[inline(always)]\n    fn __event_line_content_end("));
    assert!(source.contains("#[inline(always)]\n    fn __event_trim_whitespace_end("));
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("ab \r\n"), vec![
                StartNode(0),
                Token { kind: 1, start: 0, end: 3 },
                Token { kind: 2, start: 0, end: 2 },
                Token { kind: 3, start: 0, end: 2 },
                FinishNode,
            ]);
            assert_eq!(parse_events("\r\n"), vec![
                StartNode(0), Token { kind: 3, start: 0, end: 1 }, FinishNode,
            ]);
        },
    );
}
