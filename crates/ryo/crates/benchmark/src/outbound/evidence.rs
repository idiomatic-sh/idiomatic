//! The `evidence` port.
pub(crate) mod adapters;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
