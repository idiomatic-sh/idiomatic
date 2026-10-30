//! The `sources` port: the files tools report on, read only beneath the product's root, and the state and digests of
//! the files a tool reads.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
