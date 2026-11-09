//! The `observer` port.

pub mod adapters;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
