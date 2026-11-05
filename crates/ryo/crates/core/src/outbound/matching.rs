//! The `matching` port.

pub(crate) mod adapters;
pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
