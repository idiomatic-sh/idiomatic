//! Ruff boundary checks run without requiring an installed competitor.
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
#[cfg(test)]
mod engine_test;
#[allow(dead_code, reason = "shared reader also serves the vector host")]
#[path = "../../../../../crates/executor/src/finish/json_input.rs"]
mod json_input;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
#[path = "../../json_output.rs"]
mod json_output;
mod settings;

mod frontend_projection;
