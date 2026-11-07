//! The `analysis` port: what Tastemaker asks of ryo, the two calls and their types, and nothing else.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
