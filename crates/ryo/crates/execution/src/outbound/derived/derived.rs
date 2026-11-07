//! The `derived` port.

pub mod interface;
pub(crate) mod adapters;
pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
