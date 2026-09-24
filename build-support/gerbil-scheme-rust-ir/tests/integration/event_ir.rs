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
                  "level": {"kind": "state", "name": "level"},
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
