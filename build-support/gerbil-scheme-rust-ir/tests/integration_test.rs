//! Integration coverage for the typed Scheme-to-Rust IR boundary.

#[path = "integration/function_ir.rs"]
mod function_ir;

#[path = "integration/event_ir.rs"]
mod event_ir;

#[path = "integration/event_offset_optimization.rs"]
mod event_offset_optimization;

#[path = "integration/event_choice_ir.rs"]
mod event_choice_ir;

#[path = "integration/event_future_heading.rs"]
mod event_future_heading;

#[path = "integration/event_state_ir.rs"]
mod event_state_ir;

#[path = "integration/source_match_ir.rs"]
mod source_match_ir;
