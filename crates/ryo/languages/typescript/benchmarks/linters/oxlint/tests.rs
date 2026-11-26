//! oxlint boundary checks run without requiring an installed competitor.
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
#[allow(dead_code, reason = "shared reader also serves the vector host")]
#[path = "../../../../../crates/engine/src/executor/json_input.rs"]
mod json_input;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
mod settings;
