//! oxlint boundary checks run without requiring an installed competitor.
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
use ryo_executor::request_input as json_input;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
mod settings;
