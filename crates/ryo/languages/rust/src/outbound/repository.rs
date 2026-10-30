//! The `repository` port.

pub(crate) mod adapters;
pub(crate) mod errors;
pub(crate) mod factory;
pub(crate) mod interface;
pub(crate) mod types;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
