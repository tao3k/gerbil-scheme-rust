use gerbil_scheme_rust_ir::{
    EVENT_FUNCTION_IR_SCHEMA, compile_event_function_json, compile_ir_json,
};
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::json;
use std::{fs, process::Command, time::SystemTime};

fn stateful_document() -> serde_json::Value {
    json!({
        "schema": EVENT_FUNCTION_IR_SCHEMA,
        "name": "parse_events",
        "root_kind": 0,
        "parser_digest": format!("sha256:{}", "0".repeat(64)),
        "initial": [{"kind": "let_bool", "name": "paragraph_open", "value": false}],
        "line": [{
            "kind": "if",
            "condition": {"kind": "line_starts_with", "value": "# "},
            "consequent": [
                {"kind": "if", "condition": {"kind": "state", "name": "paragraph_open"},
                 "consequent": [{"kind": "finish_node"},
                                {"kind": "set_bool", "name": "paragraph_open",
                                 "value": {"kind": "bool", "value": false}}],
                 "alternate": []},
                {"kind": "start_node", "syntax_kind": 1},
                {"kind": "token", "syntax_kind": 3, "start": "start", "end": "end"},
                {"kind": "finish_node"}
            ],
            "alternate": [
                {"kind": "if",
                 "condition": {"kind": "not", "value": {"kind": "state", "name": "paragraph_open"}},
                 "consequent": [{"kind": "start_node", "syntax_kind": 2},
                                {"kind": "set_bool", "name": "paragraph_open",
                                 "value": {"kind": "bool", "value": true}}],
                 "alternate": []},
                {"kind": "start_node", "syntax_kind": 4},
                {"kind": "token", "syntax_kind": 5, "start": "start", "end": "end"},
                {"kind": "finish_node"}
            ]
        }],
        "finish": [{"kind": "if", "condition": {"kind": "state", "name": "paragraph_open"},
                    "consequent": [{"kind": "finish_node"}], "alternate": []}]
    })
}

#[test]
fn stateful_event_ir_typechecks_and_executes() {
    let source = compile_event_function_json(&stateful_document().to_string())
        .expect("closed event IR compiles");
    assert_eq!(
        source,
        compile_ir_json(&stateful_document().to_string()).expect("schema dispatches to events")
    );
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("# A\r\nbody\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 3, start: 0, end: 5 }, FinishNode,
                StartNode(2), StartNode(4),
                Token { kind: 5, start: 5, end: 10 }, FinishNode,
                FinishNode, FinishNode,
            ]);
        },
    );
}

fn compile_and_run(source: &str, assertions: &TokenStream) {
    let generated: TokenStream = source.parse().expect("Rust tokens parse");
    let fixture = quote! {
        #[derive(Debug, PartialEq)]
        enum TreeEvent {
            StartNode(u16),
            Token { kind: u16, start: usize, end: usize },
            FinishNode,
        }
        #generated
        #[test]
        fn generated_semantics() {
            #assertions
        }
    };
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("clock follows epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "gerbil-scheme-rust-event-ir-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("isolated rustc fixture directory");
    let input = root.join("generated.rs");
    let binary = root.join("generated-test");
    fs::write(&input, fixture.to_string()).expect("write generated fixture");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compile = Command::new(rustc)
        .arg("--edition=2024")
        .arg("--test")
        .arg(&input)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("invoke rustc");
    assert!(
        compile.status.success(),
        "generated Rust failed to typecheck:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let execute = Command::new(&binary)
        .output()
        .expect("execute generated fixture");
    assert!(
        execute.status.success(),
        "generated Rust semantics failed:\n{}",
        String::from_utf8_lossy(&execute.stdout)
    );
    fs::remove_file(binary).expect("remove generated binary");
    fs::remove_file(input).expect("remove generated fixture");
    fs::remove_dir(root).expect("remove empty fixture directory");
}

#[test]
fn level_stack_event_ir_closes_siblings_and_nested_sections() {
    let document = json!({
        "schema": EVENT_FUNCTION_IR_SCHEMA,
        "name": "parse_events",
        "root_kind": 0,
        "parser_digest": format!("sha256:{}", "1".repeat(64)),
        "initial": [
            {"kind": "let_usize", "name": "level", "value": 0},
            {"kind": "let_usize_stack", "name": "levels"}
        ],
        "line": [
            {"kind": "set_usize", "name": "level",
             "value": {"kind": "line_marker_level", "marker": 42, "separator": 32}},
            {"kind": "if",
             "condition": {"kind": "usize_positive",
                           "value": {"kind": "state", "name": "level"}},
             "consequent": [
                 {"kind": "close_through_level", "stack": "levels",
                  "level": {"kind": "state", "name": "level"}},
                 {"kind": "open_level", "stack": "levels",
                  "level": {"kind": "line_marker_level", "marker": 42,
                            "separator": 32},
                  "syntax_kind": 1},
                 {"kind": "start_node", "syntax_kind": 2},
                 {"kind": "token", "syntax_kind": 4,
                  "start": "start", "end": "end"},
                 {"kind": "finish_node"}
             ],
             "alternate": [
                 {"kind": "start_node", "syntax_kind": 3},
                 {"kind": "token", "syntax_kind": 4,
                  "start": "start", "end": "end"},
                 {"kind": "finish_node"}
             ]}
        ],
        "finish": [{"kind": "close_all_levels", "stack": "levels"}]
    });
    let source = compile_event_function_json(&document.to_string()).expect("level IR compiles");
    assert_eq!(source.matches("take_while").count(), 1);
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("* Parent\n** Child\nbody\n* Peer\n"), vec![
                StartNode(0),
                StartNode(1), StartNode(2),
                Token { kind: 4, start: 0, end: 9 }, FinishNode,
                StartNode(1), StartNode(2),
                Token { kind: 4, start: 9, end: 18 }, FinishNode,
                StartNode(3), Token { kind: 4, start: 18, end: 23 }, FinishNode,
                FinishNode, FinishNode,
                StartNode(1), StartNode(2),
                Token { kind: 4, start: 23, end: 30 }, FinishNode,
                FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn event_ir_rejects_unknown_forms_and_raw_rust() {
    let mut document = stateful_document();
    document["schema"] = json!("other.version");
    assert!(compile_event_function_json(&document.to_string()).is_err());

    let mut document = stateful_document();
    document["line"] = json!([{"kind": "raw_rust", "source": "panic!()"}]);
    assert!(compile_event_function_json(&document.to_string()).is_err());
}

#[test]
fn ascii_case_insensitive_line_prefix_is_source_backed() {
    let mut document = stateful_document();
    document["line"][0]["condition"] = json!({
        "kind": "line_starts_with_ascii_case_insensitive",
        "value": "#+begin_src"
    });
    let source = compile_event_function_json(&document.to_string()).expect("ASCII prefix IR");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("#+BeGiN_SrC rust\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 3, start: 0, end: 17 },
                FinishNode, FinishNode,
            ]);
            assert_eq!(parse_events("α#+begin_src\n"), vec![
                StartNode(0), StartNode(2), StartNode(4),
                Token { kind: 5, start: 0, end: 14 },
                FinishNode, FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn blank_line_predicate_preserves_whitespace_bytes() {
    let mut document = stateful_document();
    document["line"][0]["condition"] = json!({"kind": "line_blank"});
    let source = compile_event_function_json(&document.to_string()).expect("blank-line IR");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events(" \t\r\nα\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 3, start: 0, end: 4 }, FinishNode,
                StartNode(2), StartNode(4),
                Token { kind: 5, start: 4, end: 7 },
                FinishNode, FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn dynamic_key_line_offsets_preserve_value_and_trivia() {
    let mut document = stateful_document();
    let prefix_end = json!({"kind": "line_prefix_end", "value": "#+"});
    let key_end = json!({"kind": "line_scan_key", "from": prefix_end});
    let after_colon = json!({"kind": "line_step", "from": key_end});
    let value_start = json!({"kind": "line_skip_horizontal", "from": after_colon});
    document["line"] = json!([{
        "kind": "if",
        "condition": {"kind": "line_has_key_after_prefix", "value": "#+"},
        "consequent": [
            {"kind": "start_node", "syntax_kind": 1},
            {"kind": "token", "syntax_kind": 3, "start": "start", "end": prefix_end},
            {"kind": "token", "syntax_kind": 4, "start": prefix_end, "end": key_end},
            {"kind": "token", "syntax_kind": 3, "start": key_end, "end": value_start},
            {"kind": "token", "syntax_kind": 5, "start": value_start,
             "end": {"kind": "line_trim_end"}},
            {"kind": "token", "syntax_kind": 3,
             "start": {"kind": "line_trim_end"}, "end": "end"},
            {"kind": "finish_node"}
        ],
        "alternate": [
            {"kind": "start_node", "syntax_kind": 2},
            {"kind": "token", "syntax_kind": 5, "start": "start", "end": "end"},
            {"kind": "finish_node"}
        ]
    }]);
    document["finish"] = json!([]);
    let source = compile_event_function_json(&document.to_string()).expect("key-line IR compiles");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("#+SEQ_TODO: TODO | DONE \r\nplain\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 3, start: 0, end: 2 },
                Token { kind: 4, start: 2, end: 10 },
                Token { kind: 3, start: 10, end: 12 },
                Token { kind: 5, start: 12, end: 23 },
                Token { kind: 3, start: 23, end: 26 }, FinishNode,
                StartNode(2), Token { kind: 5, start: 26, end: 32 },
                FinishNode, FinishNode,
            ]);
            assert_eq!(parse_events("#+@bad: x\n"), vec![
                StartNode(0), StartNode(2),
                Token { kind: 5, start: 0, end: 10 },
                FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn marker_end_offset_uses_one_cached_line_level() {
    let mut document = stateful_document();
    document["line"][0]["condition"] = json!({"kind": "line_starts_with", "value": "* "});
    let marker_end = json!({"kind": "line_marker_end", "marker": 42, "separator": 32});
    document["line"][0]["consequent"][2]["end"] = marker_end.clone();
    document["line"][0]["consequent"]
        .as_array_mut()
        .unwrap()
        .insert(
            3,
            json!({"kind": "token", "syntax_kind": 5, "start": marker_end, "end": "end"}),
        );
    let source = compile_event_function_json(&document.to_string()).expect("marker offset IR");
    assert_eq!(source.matches("take_while").count(), 1);
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("* H\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 3, start: 0, end: 1 },
                Token { kind: 5, start: 1, end: 4 },
                FinishNode, FinishNode,
            ]);
        },
    );
}

#[test]
fn bounded_line_offsets_emit_source_backed_prefix_word_and_trivia() {
    let prefix = json!({"kind": "line_prefix_end", "value": "#+begin_src"});
    let word_start = json!({"kind": "line_skip_horizontal", "from": prefix});
    let word_end = json!({"kind": "line_scan_word", "from": word_start});
    let document = json!({
        "schema": EVENT_FUNCTION_IR_SCHEMA,
        "name": "parse_events",
        "root_kind": 0,
        "parser_digest": format!("sha256:{}", "2".repeat(64)),
        "initial": [],
        "line": [{"kind": "if",
          "condition": {"kind": "line_has_word_after_prefix", "value": "#+begin_src"},
          "consequent": [
            {"kind": "start_node", "syntax_kind": 1},
            {"kind": "token", "syntax_kind": 2, "start": "start", "end": prefix},
            {"kind": "token", "syntax_kind": 3, "start": prefix,
             "end": word_start},
            {"kind": "token", "syntax_kind": 4, "start": word_start,
             "end": word_end},
            {"kind": "token", "syntax_kind": 3, "start": word_end,
             "end": "end"},
            {"kind": "finish_node"}
          ],
          "alternate": [{"kind": "token", "syntax_kind": 5,
                         "start": "start", "end": "end"}]
        }],
        "finish": []
    });
    let source = compile_event_function_json(&document.to_string()).expect("bounded offset IR");
    compile_and_run(
        &source,
        &quote! {
            use TreeEvent::{FinishNode, StartNode, Token};
            assert_eq!(parse_events("#+BeGiN_SrC rust :x\n"), vec![
                StartNode(0), StartNode(1),
                Token { kind: 2, start: 0, end: 11 },
                Token { kind: 3, start: 11, end: 12 },
                Token { kind: 4, start: 12, end: 16 },
                Token { kind: 3, start: 16, end: 20 },
                FinishNode, FinishNode,
            ]);
            assert_eq!(parse_events("#+begin_src \n"), vec![
                StartNode(0), Token { kind: 5, start: 0, end: 13 }, FinishNode,
            ]);
        },
    );
}
