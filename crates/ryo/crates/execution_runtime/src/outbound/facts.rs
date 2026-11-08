//! The `facts` port.

pub mod interface;
pub mod types;
pub mod errors;
pub(crate) mod adapters;
pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
