//! The `process_meter` port: this process's own CPU time and peak resident memory.

pub(crate) mod adapters;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
