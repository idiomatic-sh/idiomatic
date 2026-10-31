//! Execution boundary.
pub(crate) mod adapters;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub mod factory;
