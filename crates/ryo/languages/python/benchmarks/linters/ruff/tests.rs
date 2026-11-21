//! Ruff boundary checks run without requiring an installed competitor.
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
#[path = "cost_test.rs"]
mod cost_test;
#[cfg(test)]
mod engine_test;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
mod settings;

mod frontend_projection;
