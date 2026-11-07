//! Suite boundary.
pub(crate) mod adapters;
pub mod interface;

pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

mod constants;
pub mod types;
