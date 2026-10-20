use gerbil_scheme_rust_ir::{FUNCTION_IR_SCHEMA, compile_function_json};
use serde_json::{Value, json};

fn document() -> Value {
    json!({"schema": FUNCTION_IR_SCHEMA, "name": "capture", "parameters": [{"name":"value", "ty":"String"}],
        "result":"impl Fn() -> String + Send + Sync + 'static",
        "body":{"bindings":[], "result":{"kind":"closure", "parameters":[], "result":"String",
            "move_capture":true, "body":{"bindings":[], "result":{"kind":"method", "receiver":{"kind":"name","value":"value"}, "method":"clone", "arguments":[]}}}}})
}
#[test]
fn owned_closure_ir_is_typed_and_rejects_raw_body_or_duplicate_parameters() {
    let source = compile_function_json(&document().to_string()).unwrap();
    let parsed: syn::ItemFn = syn::parse_str(&source).unwrap();
    assert!(matches!(parsed.sig.output, syn::ReturnType::Type(_, _)));
    let mut duplicate = document();
    duplicate["body"]["result"]["parameters"] =
        json!([{"name":"x","ty":"String"}, {"name":"x","ty":"String"}]);
    assert!(compile_function_json(&duplicate.to_string()).is_err());
    let mut raw = document();
    raw["body"]["result"]["body"]["result"] = json!({"kind":"raw_rust","source":"panic!()"});
    assert!(compile_function_json(&raw.to_string()).is_err());
    let mut unknown = document();
    unknown["body"]["result"]["fallback"] = json!(true);
    assert!(compile_function_json(&unknown.to_string()).is_err());
}

#[test]
fn compiled_closures_capture_state_and_remain_real_rust_callables() {
    use std::{fs, process::Command};
    let root = std::env::temp_dir().join(format!(
        "gsr-closure-ir-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let function = compile_function_json(&document().to_string()).unwrap();
    let mut reader = document();
    reader["name"] = json!("reader");
    reader["parameters"] =
        json!([{"name":"state", "ty":"std::sync::Arc<std::sync::RwLock<String>>"}]);
    reader["body"]["result"]["body"]["result"] = json!({"kind":"method", "method":"clone", "arguments":[],
        "receiver":{"kind":"method","method":"unwrap","arguments":[],
            "receiver":{"kind":"method","method":"read","arguments":[], "receiver":{"kind":"name","value":"state"}}}});
    let reader = compile_function_json(&reader.to_string()).unwrap();
    fs::write(
        root.join("generated.rs"),
        format!(
            "{function}\n{reader}\n#[test] fn captured_semantics() {{
        let state = std::sync::Arc::new(std::sync::RwLock::new(String::from(\"old\")));
        let captured = capture(state.read().unwrap().clone()); let live = reader(state.clone());
        *state.write().unwrap() = String::from(\"new\");
        assert_eq!(captured(), \"old\"); assert_eq!(live(), \"new\");
        let other = capture(String::from(\"independent\"));
        assert_eq!(std::thread::spawn(other).join().unwrap(), \"independent\");
    }}"
        ),
    )
    .unwrap();
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--test"])
        .arg(root.join("generated.rs"))
        .arg("-o")
        .arg(root.join("generated-test"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(root.join("generated-test")).output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).unwrap();
}
