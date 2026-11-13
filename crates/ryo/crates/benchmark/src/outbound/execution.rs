//! Execution boundary.
pub(crate) mod adapters;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub mod factory;

pub mod errors;
