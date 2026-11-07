//! The `notes` port.

pub(crate) mod adapters;
pub(crate) mod errors;
pub(crate) mod factory;
pub(crate) mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
