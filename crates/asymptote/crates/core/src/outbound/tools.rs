//! The `tools` port: running a configured tool, telling a tool that ran and failed from one that could not start.

pub(crate) mod adapters;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
