//! The `manifest_reading` port.

pub(crate) mod adapters;
pub(crate) mod errors;
pub(crate) mod factory;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
pub(crate) mod interface;
