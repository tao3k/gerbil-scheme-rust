//! Generic future-marker cache semantics and shared builder lowering.

use super::event_ir::{compile_and_run, stateful_document};
use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

#[test]
fn future_line_marker_stops_at_heading_and_parent_boundary() {
    let mut document = stateful_document();
    document["line"] = json!([{
        "kind": "if",
        "condition": {
            "kind": "future_line_marker_before_boundary",
            "target": "#+END_QUOTE",
            "stop": "#+END_CENTER",
            "heading_marker": 42,
            "heading_separator": 32,
            "indent": true,
            "stop_at_heading": true
        },
        "consequent": [{"kind": "start_node", "syntax_kind": 1}, {"kind": "finish_node"}],
        "alternate": [{"kind": "start_node", "syntax_kind": 2}, {"kind": "finish_node"}]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("future source-line marker is typed event IR");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode};
            assert_eq!(parse_events("#+BEGIN_QUOTE\n  #+end_quote\n"), vec![
                StartNode(0), StartNode(1), FinishNode,
                StartNode(2), FinishNode, FinishNode,
            ]);
            assert_eq!(parse_events("#+BEGIN_QUOTE\n** Next\n#+END_QUOTE\n"), vec![
                StartNode(0), StartNode(2), FinishNode,
                StartNode(1), FinishNode,
                StartNode(2), FinishNode, FinishNode,
            ]);
            assert_eq!(parse_events("#+BEGIN_QUOTE\n#+END_CENTER\n#+END_QUOTE\n"), vec![
                StartNode(0), StartNode(2), FinishNode,
                StartNode(1), FinishNode,
                StartNode(2), FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn distinct_future_markers_share_one_index_builder() {
    let mut document = stateful_document();
    let condition = json!({
        "kind": "future_line_marker_before_boundary",
        "target": "#+END_QUOTE",
        "stop": "",
        "heading_marker": 42,
        "heading_separator": 32,
        "indent": true,
        "stop_at_heading": true
    });
    document["line"] = json!([
        {"kind": "if", "condition": condition, "consequent": [], "alternate": []},
        {"kind": "if", "condition": condition, "consequent": [], "alternate": []},
        {"kind": "if", "condition": {
            "kind": "future_line_marker_before_boundary",
            "target": "#+END_CENTER", "stop": "", "heading_marker": 42,
            "heading_separator": 32, "indent": false, "stop_at_heading": false
        }, "consequent": [], "alternate": []}
    ]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("repeated future marker is typed event IR");
    assert_eq!(source.matches("__event_future_build {").count(), 1);
    assert_eq!(source.matches("get_or_init").count(), 3);
    compile_and_run(
        &source,
        &quote! {
            assert!(!parse_events("#+BEGIN_QUOTE\n#+END_QUOTE\n").is_empty());
        },
    );
}

#[test]
fn future_line_marker_requires_declared_key_value_body() {
    let mut document = stateful_document();
    document["line"] = json!([{
        "kind": "if",
        "condition": {
            "kind": "future_line_marker_before_boundary",
            "target": ":END:",
            "stop": "",
            "heading_marker": 42,
            "heading_separator": 32,
            "indent": true,
            "stop_at_heading": true,
            "body_key_marker": 58
        },
        "consequent": [{"kind": "start_node", "syntax_kind": 1}, {"kind": "finish_node"}],
        "alternate": [{"kind": "start_node", "syntax_kind": 2}, {"kind": "finish_node"}]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("future key-value body is typed event IR");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode};
            assert!(matches!(parse_events(":PROPERTIES:\n:ID: x\n:END:\n")[1], StartNode(1)));
            assert!(matches!(parse_events(":PROPERTIES:\n:header-args:python: :session local\n:END:\n")[1], StartNode(1)));
            assert!(matches!(parse_events(":PROPERTIES:\n:ID: value:more\n:END:\n")[1], StartNode(1)));
            assert!(matches!(parse_events(":PROPERTIES:\nmalformed\n:END:\n")[1], StartNode(2)));
            assert!(matches!(parse_events(":PROPERTIES:\n:header args:python: x\n:END:\n")[1], StartNode(2)));
        },
    );
}
