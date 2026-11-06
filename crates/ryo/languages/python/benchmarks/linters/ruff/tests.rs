//! Ruff boundary checks run without requiring an installed competitor.
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
mod engine_test;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
mod settings;

mod host_policy;

mod frontend_projection;
