//! The `sarif` port: what a tool's SARIF 2.1.0 log reports, and the log `check --format sarif` prints.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
