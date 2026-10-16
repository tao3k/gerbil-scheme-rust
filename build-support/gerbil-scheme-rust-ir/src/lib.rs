//! Typed build-time IR for Scheme algorithms compiled into Rust functions.

mod compiler;
mod dispatch;
mod event_compiler;
mod source_match_compiler;

pub use compiler::{
    BinaryOperator, BindingIr, BlockIr, CompileError, ExprIr, FUNCTION_IR_SCHEMA, FunctionIr,
    ParameterIr, compile_function, compile_function_json,
};
pub use dispatch::compile_ir_json;
pub use event_compiler::{
    EVENT_FUNCTION_IR_SCHEMA, EventBoundaryIr, EventComputedOffsetIr, EventFunctionIr,
    EventOffsetIr, EventPredicateIr, EventStateSlot, EventStatementIr, EventUsizeParameterIr,
    compile_event_function, compile_event_function_json,
};
pub use source_match_compiler::{
    SOURCE_MATCH_IR_SCHEMA, SourceBoundaryIr, SourceCandidateIr, SourceMatchIr, SourceScanIr,
    SourceWinnerIr, compile_source_match, compile_source_match_json,
};

#[cfg(test)]
asp_rust::asp_rust_cargo_test_gate!(
    config = {
        asp_rust::default_asp_rust_config()
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/compiler.rs",
                [asp_rust::RustOwnerResponsibility::PureDomainLogic],
            ))
            .with_verification_profile_hint(asp_rust::RustVerificationProfileHint::new(
                "src/lib.rs",
                [asp_rust::RustOwnerResponsibility::PublicApi],
            ))
    }
);
