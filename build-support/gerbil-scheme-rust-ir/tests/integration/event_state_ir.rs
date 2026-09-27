//! Named lookahead and state-stack event transitions.

use gerbil_scheme_rust_ir::compile_event_function_json;
use quote::quote;
use serde_json::json;

use super::event_ir::{compile_and_run, stateful_document};

#[test]
fn typed_parameter_flows_into_source_local_helper() {
    let mut document = stateful_document();
    document["initial"] = json!([
        {"kind": "let_bool", "name": "paragraph_open", "value": false},
        {"kind": "let_usize", "name": "threshold", "value": 2}
    ]);
    document["parameters"] = json!([{
        "name": "configured_threshold", "state": "threshold", "default": 2
    }]);
    document["line"] = json!([{
        "kind": "call_source_helper", "name": "local_span",
        "from": "start", "until": "end",
        "arguments": [{"kind": "state", "name": "threshold"}]
    }]);
    document["helpers"] = json!([{
        "name": "local_span",
        "initial": [{"kind": "let_usize", "name": "threshold", "value": 2}],
        "parameters": ["threshold"],
        "body": [{
            "kind": "if",
            "condition": {"kind": "usize_equal",
                          "left": {"kind": "state", "name": "threshold"},
                          "right": {"kind": "usize", "value": 2}},
            "consequent": [{"kind": "token", "syntax_kind": 1,
                            "start": "start", "end": "end"}],
            "alternate": [{"kind": "token", "syntax_kind": 2,
                           "start": "start", "end": "end"}]
        }]
    }]);
    let source = compile_event_function_json(&document.to_string())
        .expect("typed helper parameter compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::Token;
            assert!(parse_events("x\n").iter().any(|event| matches!(event, Token { kind: 1, .. })));
            assert!(parse_events_with_parameters("x\n", 5)
                .iter().any(|event| matches!(event, Token { kind: 2, .. })));
        },
    );
    document["line"][0]["arguments"] = json!([]);
    assert!(compile_event_function_json(&document.to_string()).is_err());
    document["line"][0]["arguments"] = json!([{"kind": "state", "name": "threshold"}]);
    document["helpers"][0]["parameters"] = json!(["missing"]);
    assert!(compile_event_function_json(&document.to_string()).is_err());
}

#[test]
fn typed_unsigned_parameter_overrides_declared_state_without_a_second_parser() {
    let mut document = stateful_document();
    document["initial"]
        .as_array_mut()
        .expect("initial state declarations")
        .push(json!({"kind": "let_usize", "name": "threshold", "value": 15}));
    document["parameters"] = json!([{
        "name": "configured_threshold", "state": "threshold", "default": 15
    }]);
    document["line"] = json!([{
        "kind": "if",
        "condition": {"kind": "usize_equal",
                      "left": {"kind": "state", "name": "threshold"},
                      "right": {"kind": "usize", "value": 7}},
        "consequent": [{"kind": "token", "syntax_kind": 1,
                        "start": "start", "end": "end"}],
        "alternate": [{"kind": "token", "syntax_kind": 2,
                      "start": "start", "end": "end"}]
    }]);
    let source =
        compile_event_function_json(&document.to_string()).expect("typed event parameter compiles");
    compile_and_run(
        &source,
        &quote! {
            let first_kind = |events: Vec<TreeEvent>| {
                events.into_iter().find_map(|event| match event {
                    TreeEvent::Token { kind, .. } => Some(kind),
                    _ => None,
                })
            };
            assert_eq!(first_kind(parse_events("x\n")), Some(2));
            assert_eq!(first_kind(parse_events_with_parameters("x\n", 7)), Some(1));
        },
    );
    document["parameters"][0]["state"] = json!("missing_state");
    assert!(compile_event_function_json(&document.to_string()).is_err());
    document["parameters"][0]["state"] = json!("threshold");
    document["parameters"][0]["default"] = json!(16);
    assert!(compile_event_function_json(&document.to_string()).is_err());
    document["parameters"][0]["default"] = json!(15);
    document["parameters"][0]["name"] = json!("threshold");
    assert!(compile_event_function_json(&document.to_string()).is_err());
    document["parameters"][0]["name"] = json!("configured_threshold");
    document["line"]
        .as_array_mut()
        .expect("line transitions")
        .push(json!({
            "kind": "set_usize", "name": "threshold",
            "value": {"kind": "usize", "value": 8}
        }));
    let mutable_source = compile_event_function_json(&document.to_string())
        .expect("mutable configured state compiles");
    compile_and_run(
        &mutable_source,
        &quote! {
            assert!(!parse_events_with_parameters("x\n", 7).is_empty());
        },
    );
}

#[test]
fn future_named_marker_matches_saved_name_before_heading_or_parent_boundary() {
    let mut document = stateful_document();
    let name_from = json!({"kind": "line_prefix_end", "value": "#+BEGIN_"});
    let name_until = json!({"kind": "line_scan_key", "from": name_from});
    document["line"] = json!([{
        "kind": "if",
        "condition": {"kind": "line_starts_with_ascii_case_insensitive",
                      "value": "#+BEGIN_"},
        "consequent": [{
            "kind": "if",
            "condition": {
                "kind": "future_named_line_marker_before_boundary",
                "name_from": name_from, "name_until": name_until,
                "target_prefix": "#+END_", "target_suffix": "",
                "stop": "#+END_CENTER", "heading_marker": 42,
                "heading_separator": 32, "indent": true,
                "stop_at_heading": true, "ascii_case_insensitive": true
            },
            "consequent": [{"kind": "token", "syntax_kind": 1,
                            "start": "start", "end": "end"}],
            "alternate": [{"kind": "token", "syntax_kind": 2,
                           "start": "start", "end": "end"}]
        }],
        "alternate": [{"kind": "token", "syntax_kind": 3,
                       "start": "start", "end": "end"}]
    }]);
    document["finish"] = json!([]);
    let source =
        compile_event_function_json(&document.to_string()).expect("named future marker compiles");
    compile_and_run(
        &source,
        &quote! {
            let input = "#+BEGIN_foo\n#+end_FOO\n#+BEGIN_bar\n* Next\n#+END_bar\n#+BEGIN_baz\n#+END_CENTER\n#+END_baz\n#+BEGIN_qux\n#+END_other\n#+END_QUX\n";
            let opener_kinds = parse_events(input).into_iter().filter_map(|event| {
                match event {
                    TreeEvent::Token { kind, .. } if matches!(kind, 1 | 2) =>
                        Some(kind),
                    _ => None,
                }
            }).collect::<Vec<_>>();
            assert_eq!(opener_kinds, vec![1, 2, 2, 1]);
        },
    );
}

#[test]
fn future_named_marker_stops_before_a_named_parent_close() {
    let mut document = stateful_document();
    let name_from = json!({"kind": "line_prefix_end", "value": "#+BEGIN_"});
    let name_until = json!({"kind": "line_scan_key", "from": name_from});
    document["initial"] = json!([
        {"kind": "let_usize", "name": "parent_start", "value": 0},
        {"kind": "let_usize", "name": "parent_end", "value": 0}
    ]);
    document["line"] = json!([{
        "kind": "if",
        "condition": {"kind": "line_starts_with", "value": "#+BEGIN_OUTER"},
        "consequent": [
            {"kind": "set_usize", "name": "parent_start",
             "value": {"kind": "offset", "value": name_from}},
            {"kind": "set_usize", "name": "parent_end",
             "value": {"kind": "offset", "value": name_until}}
        ],
        "alternate": [{
            "kind": "if",
            "condition": {"kind": "line_starts_with", "value": "#+BEGIN_INNER"},
            "consequent": [{
                "kind": "if",
                "condition": {
                    "kind": "future_named_line_marker_before_boundary",
                    "name_from": name_from, "name_until": name_until,
                    "target_prefix": "#+END_", "target_suffix": "",
                    "stop": "", "heading_marker": 42, "heading_separator": 32,
                    "indent": true, "stop_at_heading": true,
                    "ascii_case_insensitive": true,
                    "stop_name_from": {"kind": "state_offset", "name": "parent_start"},
                    "stop_name_until": {"kind": "state_offset", "name": "parent_end"},
                    "stop_prefix": "#+END_", "stop_suffix": "",
                    "stop_ascii_case_insensitive": true
                },
                "consequent": [{"kind": "token", "syntax_kind": 1,
                                "start": "start", "end": "end"}],
                "alternate": [{"kind": "token", "syntax_kind": 2,
                               "start": "start", "end": "end"}]
            }],
            "alternate": [{"kind": "token", "syntax_kind": 3,
                           "start": "start", "end": "end"}]
        }]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string())
        .expect("dynamic named parent boundary compiles");
    compile_and_run(
        &source,
        &quote! {
            let closed_parent_first =
                parse_events("#+BEGIN_OUTER\n#+BEGIN_INNER\n#+END_outer\n#+END_inner\n");
            let closed_child_first =
                parse_events("#+BEGIN_OUTER\n#+BEGIN_INNER\n#+END_inner\n#+END_outer\n");
            let opener_kind = |events: Vec<TreeEvent>| {
                events.into_iter().find_map(|event| match event {
                    TreeEvent::Token { kind, .. } if matches!(kind, 1 | 2) => Some(kind),
                    _ => None,
                })
            };
            assert_eq!(opener_kind(closed_parent_first), Some(2));
            assert_eq!(opener_kind(closed_child_first), Some(1));
        },
    );
}

#[test]
fn pop_frame_discards_parser_state_without_closing_syntax_nodes() {
    let mut document = stateful_document();
    document["initial"] = json!([
        {"kind": "let_usize_stack", "name": "saved"}
    ]);
    document["line"] = json!([
        {"kind": "push_frame", "stack": "saved",
         "value": {"kind": "usize", "value": 7}},
        {"kind": "pop_frame", "stack": "saved"},
        {"kind": "if", "condition": {"kind": "stack_nonempty", "stack": "saved"},
         "consequent": [{"kind": "start_node", "syntax_kind": 1},
                        {"kind": "finish_node"}],
         "alternate": [{"kind": "token", "syntax_kind": 3,
                        "start": "start", "end": "end"}]}
    ]);
    document["finish"] = json!([]);
    let source =
        compile_event_function_json(&document.to_string()).expect("state-only frame pop compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("a\n"), vec![
                StartNode(0), Token { kind: 3, start: 0, end: 2 }, FinishNode,
            ]);
        },
    );
}

#[test]
fn close_frame_emits_one_node_close_even_for_equal_nested_frames() {
    let mut document = stateful_document();
    document["initial"] = json!([
        {"kind": "let_usize_stack", "name": "frames"}
    ]);
    document["line"] = json!([
        {"kind": "start_node", "syntax_kind": 1},
        {"kind": "push_frame", "stack": "frames",
         "value": {"kind": "usize", "value": 7}},
        {"kind": "start_node", "syntax_kind": 2},
        {"kind": "push_frame", "stack": "frames",
         "value": {"kind": "usize", "value": 7}},
        {"kind": "close_frame", "stack": "frames", "finish_count": 1},
        {"kind": "close_frame", "stack": "frames", "finish_count": 1}
    ]);
    document["finish"] = json!([]);
    let source =
        compile_event_function_json(&document.to_string()).expect("single-frame close compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode};
            assert_eq!(parse_events("a\n"), vec![
                StartNode(0), StartNode(1), StartNode(2),
                FinishNode, FinishNode, FinishNode,
            ]);
        },
    );
}
