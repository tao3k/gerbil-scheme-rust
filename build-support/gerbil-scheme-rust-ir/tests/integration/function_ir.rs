use gerbil_scheme_rust_ir::{FUNCTION_IR_SCHEMA, compile_function_json};
use serde_json::json;

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
