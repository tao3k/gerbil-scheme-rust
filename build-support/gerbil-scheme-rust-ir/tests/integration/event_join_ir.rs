//! A join emits the shared continuation once after bounded branch selection.

use super::event_ir::{compile_and_run, stateful_document};
use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

#[test]
fn join_once_preserves_handled_and_fallthrough_paths() {
    let mut document = stateful_document();
    document["initial"] = json!([]);
    document["line"] = json!([{
        "kind": "join_once", "handled": "handled",
        "branches": [{
            "kind": "if", "condition": {"kind": "line_starts_with", "value": "#"},
            "consequent": [
                {"kind": "token", "syntax_kind": 1, "start": "start", "end": "end"},
                {"kind": "set_bool", "name": "handled",
                 "value": {"kind": "bool", "value": true}}
            ], "alternate": []
        }],
        "fallback": [{"kind": "token", "syntax_kind": 2,
                      "start": "start", "end": "end"}]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string()).expect("join IR compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("# a\ntext\n"), vec![
                StartNode(0),
                Token { kind: 1, start: 0, end: 4 },
                Token { kind: 2, start: 4, end: 9 },
                FinishNode,
            ]);
        },
    );
}
