//! The gated operations of a linter bench (correctness, measure and parity) and where each writes.
pub(crate) mod constants;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
