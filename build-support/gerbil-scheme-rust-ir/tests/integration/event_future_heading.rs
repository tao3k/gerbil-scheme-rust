//! Generic future-heading title semantics and cached Rust lowering.

use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

use super::event_ir::{compile_and_run, stateful_document};

#[test]
fn future_heading_title_matches_level_and_exact_title_once() {
    let mut document = stateful_document();
    let condition = json!({
        "kind": "future_heading_title",
        "heading_marker": 42,
        "heading_separator": 32,
        "min_level": {"kind": "usize", "value": 4},
        "title": "END"
    });
    document["line"] = json!([{
        "kind": "if",
        "condition": condition,
        "consequent": [{"kind": "start_node", "syntax_kind": 1}, {"kind": "finish_node"}],
        "alternate": [{"kind": "start_node", "syntax_kind": 2}, {"kind": "finish_node"}]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("future heading title is typed event IR");
    assert_eq!(source.matches("fn __event_heading_future_").count(), 1);
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::StartNode;
            let later = parse_events("**** Task\n* Outline\n***** END  \r\n");
            assert!(matches!(later[1], StartNode(1)));
            let shallow = parse_events("**** Task\n*** END\n");
            assert!(matches!(shallow[1], StartNode(2)));
            let suffix = parse_events("**** Task\n**** END extra\n");
            assert!(matches!(suffix[1], StartNode(2)));
            let immediate = parse_events("**** END\n");
            assert!(matches!(immediate[1], StartNode(2)));
        },
    );
}

#[test]
fn future_heading_title_rejects_non_heading_declarations() {
    let mut document = stateful_document();
    document["line"] = json!([{
        "kind": "if",
        "condition": {
            "kind": "future_heading_title",
            "heading_marker": 42,
            "heading_separator": 32,
            "min_level": {"kind": "usize", "value": 0},
            "title": "END"
        },
        "consequent": [],
        "alternate": []
    }]);
    assert!(compile_event_function_json(&document.to_string()).is_err());
}

#[test]
fn future_heading_title_uses_a_typed_runtime_threshold() {
    let mut document = stateful_document();
    document["initial"]
        .as_array_mut()
        .expect("initial state")
        .push(json!({"kind": "let_usize", "name": "heading_limit", "value": 4}));
    document["parameters"] = json!([{
        "name": "configured_limit", "state": "heading_limit", "default": 4
    }]);
    document["line"] = json!([{
        "kind": "if",
        "condition": {
            "kind": "future_heading_title", "heading_marker": 42,
            "heading_separator": 32,
            "min_level": {"kind": "state", "name": "heading_limit"},
            "title": "END"
        },
        "consequent": [{"kind": "start_node", "syntax_kind": 1},
                       {"kind": "finish_node"}],
        "alternate": [{"kind": "start_node", "syntax_kind": 2},
                      {"kind": "finish_node"}]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("runtime heading threshold compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::StartNode;
            let input = "**** Task\n***** END\n";
            assert!(matches!(parse_events(input)[1], StartNode(1)));
            assert!(matches!(parse_events_with_parameters(input, 6)[1], StartNode(2)));
        },
    );
}
