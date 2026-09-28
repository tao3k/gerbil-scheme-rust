//! Exclusive event-branch lowering and execution.

use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

use super::event_ir::{compile_and_run, stateful_document};

#[test]
fn exclusive_event_choices_lower_to_flat_else_if() {
    let mut document = stateful_document();
    document["initial"] = json!([]);
    document["finish"] = json!([]);
    document["line"] = json!([{
        "kind": "if",
        "condition": {"kind": "line_starts_with", "value": "#"},
        "consequent": [{"kind": "start_node", "syntax_kind": 1},
                       {"kind": "finish_node"}],
        "alternate": [{
            "kind": "if",
            "condition": {"kind": "line_starts_with", "value": "!"},
            "consequent": [{"kind": "start_node", "syntax_kind": 2},
                           {"kind": "finish_node"}],
            "alternate": [{"kind": "start_node", "syntax_kind": 3},
                          {"kind": "finish_node"}]
        }]
    }]);
    let source = compile_event_function_json(&document.to_string()).expect("choice IR compiles");
    assert!(source.contains("else if"));
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode};
            assert_eq!(parse_events("# a\n! b\nplain\n"), vec![
                StartNode(0), StartNode(1), FinishNode,
                StartNode(2), FinishNode,
                StartNode(3), FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn repeated_byte_set_checks_share_pure_generated_functions() {
    let mut document = stateful_document();
    document["initial"] = json!([]);
    document["finish"] = json!([]);
    document["line"] = json!([
        {"kind": "if",
         "condition": {"kind": "line_bytes_any_in", "from": "start",
                       "until": {"kind": "line_content_end"}, "values": [97]},
         "consequent": [{"kind": "start_node", "syntax_kind": 1},
                        {"kind": "finish_node"}],
         "alternate": []},
        {"kind": "if",
         "condition": {"kind": "line_bytes_any_in", "from": "start",
                       "until": {"kind": "line_content_end"}, "values": [97]},
         "consequent": [{"kind": "start_node", "syntax_kind": 2},
                        {"kind": "finish_node"}],
         "alternate": []},
        {"kind": "if",
         "condition": {"kind": "line_bytes_all_in", "from": "start",
                       "until": {"kind": "line_content_end"}, "values": [97]},
         "consequent": [{"kind": "start_node", "syntax_kind": 3},
                        {"kind": "finish_node"}],
         "alternate": []}
    ]);
    let source = compile_event_function_json(&document.to_string()).expect("byte sets compile");
    assert_eq!(source.matches("fn __event_any_byte_in(").count(), 1);
    assert_eq!(source.matches("fn __event_all_bytes_in(").count(), 1);
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode};
            assert_eq!(parse_events("a\nb\n"), vec![
                StartNode(0), StartNode(1), FinishNode,
                StartNode(2), FinishNode,
                StartNode(3), FinishNode, FinishNode,
            ]);
        },
    );
}
