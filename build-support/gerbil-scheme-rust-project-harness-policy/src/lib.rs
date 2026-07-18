//! Project-local Rust harness policy for the Gerbil Scheme bindings workspace.

mod config;

pub use config::{
    assert_gerbil_scheme_rust_project_harness_gate_from_env,
    rust_project_harness_policy_for_project,
};
pub use rust_lang_project_harness::{
    RustProjectHarnessDownstreamPolicy, RustProjectHarnessDownstreamPolicyReceipt,
};
