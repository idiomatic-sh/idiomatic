//! Suite boundary.
pub(crate) mod adapters;
pub mod errors;
pub mod interface;

pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub mod types;
