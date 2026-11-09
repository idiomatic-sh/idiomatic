//! The `observer` port.

pub(crate) mod adapters;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
