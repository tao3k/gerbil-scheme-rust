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
