//! Versioned IR dispatch across closed Scheme-to-Rust backends.

use crate::{
    CompileError, EVENT_FUNCTION_IR_SCHEMA, FUNCTION_IR_SCHEMA, compile_event_function_json,
    compile_function_json,
};

/// Dispatch one versioned Scheme IR document to its closed Rust backend.
///
/// # Errors
/// Rejects malformed JSON, missing or unknown schemas, and invalid syntax.
pub fn compile_ir_json(input: &str) -> Result<String, CompileError> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(CompileError::Json)?;
    match value.get("schema").and_then(serde_json::Value::as_str) {
        Some(FUNCTION_IR_SCHEMA) => compile_function_json(input),
        Some(EVENT_FUNCTION_IR_SCHEMA) => compile_event_function_json(input),
        Some(other) => Err(CompileError::Schema(other.to_owned())),
        None => Err(CompileError::Schema("missing schema".to_owned())),
    }
}
