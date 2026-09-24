use gerbil_scheme_rust_ir::{FUNCTION_IR_SCHEMA, compile_function_json};
use serde_json::json;
use std::{fs, process::Command, time::SystemTime};

#[test]
fn typed_ir_compiles_to_a_rust_function() {
    let document = json!({
        "schema": FUNCTION_IR_SCHEMA,
        "name": "headline_content",
        "parameters": [{"name": "title", "ty": "&str"}],
        "result": "String",
        "body": {
            "bindings": [],
            "result": {
                "kind": "method",
                "receiver": {
                    "kind": "method",
                    "receiver": {
                        "kind": "tuple_index",
                        "tuple": {
                            "kind": "method",
                            "receiver": {
                                "kind": "method",
                                "receiver": {
                                    "kind": "method",
                                    "receiver": {"kind": "name", "value": "title"},
                                    "method": "trim",
                                    "arguments": []
                                },
                                "method": "split_once",
                                "arguments": [{"kind": "name", "value": "char::is_whitespace"}]
                            },
                            "method": "unwrap_or_default",
                            "arguments": []
                        },
                        "index": 1
                    },
                    "method": "trim",
                    "arguments": []
                },
                "method": "to_owned",
                "arguments": []
            }
        }
    });
    let source = compile_function_json(&document.to_string()).expect("typed IR must compile");
    let function: syn::ItemFn = syn::parse_str(&source).expect("Rust source must parse");
    assert_eq!(function.sig.ident, "headline_content");
    assert_eq!(function.sig.inputs.len(), 1);
}

#[test]
fn unknown_schema_and_raw_rust_snippets_fail_closed() {
    let document = json!({
        "schema": "other.version",
        "name": "demo",
        "parameters": [],
        "result": "String",
        "body": {"bindings": [], "result": {"kind": "string", "value": "value"}}
    });
    assert!(compile_function_json(&document.to_string()).is_err());

    let document = json!({
        "schema": FUNCTION_IR_SCHEMA,
        "name": "demo",
        "parameters": [],
        "result": "String",
        "body": {"bindings": [], "result": {"kind": "raw_rust", "source": "panic!()"}}
    });
    assert!(compile_function_json(&document.to_string()).is_err());
}

#[test]
fn nested_pure_bindings_compile_as_lexical_rust_blocks() {
    let document = json!({
        "schema": FUNCTION_IR_SCHEMA,
        "name": "has_prefix",
        "parameters": [{"name": "source", "ty": "&str"}],
        "result": "bool",
        "body": {
            "bindings": [],
            "result": {
                "kind": "block",
                "bindings": [{
                    "name": "head",
                    "value": {"kind": "first_word", "value": {"kind": "name", "value": "source"}}
                }],
                "result": {
                    "kind": "binary",
                    "operator": "equal",
                    "left": {"kind": "name", "value": "head"},
                    "right": {"kind": "string", "value": "TODO"}
                }
            }
        }
    });
    let source = compile_function_json(&document.to_string()).expect("nested IR must compile");
    let function: syn::ItemFn = syn::parse_str(&source).expect("Rust source must parse");
    assert!(matches!(
        function.block.stmts.first(),
        Some(syn::Stmt::Expr(syn::Expr::Block(_), _))
    ));
}

#[test]
fn generated_function_typechecks_and_executes_with_rustc() {
    let document = json!({
        "schema": FUNCTION_IR_SCHEMA,
        "name": "owned_rest_after_first_word",
        "parameters": [{"name": "input", "ty": "&str"}],
        "result": "String",
        "body": {
            "bindings": [],
            "result": {
                "kind": "method",
                "receiver": {
                    "kind": "method",
                    "receiver": {
                        "kind": "after",
                        "value": {
                            "kind": "method",
                            "receiver": {"kind": "name", "value": "input"},
                            "method": "trim",
                            "arguments": []
                        },
                        "delimiter": {"kind": "name", "value": "char::is_whitespace"}
                    },
                    "method": "trim",
                    "arguments": []
                },
                "method": "to_owned",
                "arguments": []
            }
        }
    });
    let function = compile_function_json(&document.to_string()).expect("typed IR must compile");
    let source = format!(
        "{function}\n#[test]\nfn generated_semantics() {{\n    assert_eq!(owned_rest_after_first_word(\"  WAIT   [#A] Head :tag:  \"), \"[#A] Head :tag:\");\n    assert_eq!(owned_rest_after_first_word(\"WAIT\"), \"\");\n}}\n"
    );
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "gerbil-scheme-rust-ir-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("create isolated rustc fixture directory");
    let input = root.join("generated.rs");
    let binary = root.join("generated-test");
    fs::write(&input, source).expect("write generated Rust fixture");

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compile = Command::new(rustc)
        .arg("--edition=2024")
        .arg("--test")
        .arg(&input)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("invoke rustc for the generated fixture");
    assert!(
        compile.status.success(),
        "generated Rust failed to typecheck:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let execute = Command::new(&binary)
        .output()
        .expect("execute generated Rust fixture");
    assert!(
        execute.status.success(),
        "generated Rust semantics failed:\n{}",
        String::from_utf8_lossy(&execute.stdout)
    );

    fs::remove_file(binary).expect("remove generated test binary");
    fs::remove_file(input).expect("remove generated Rust fixture");
    fs::remove_dir(root).expect("remove empty rustc fixture directory");
}
