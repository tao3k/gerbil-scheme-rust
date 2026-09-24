//! Typed build-time IR for Scheme algorithms compiled into Rust functions.

mod compiler;

pub use compiler::{
    BinaryOperator, BindingIr, BlockIr, CompileError, ExprIr, FUNCTION_IR_SCHEMA, FunctionIr,
    ParameterIr, compile_function, compile_function_json,
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
