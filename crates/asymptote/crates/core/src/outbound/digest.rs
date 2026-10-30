//! The `digest` port: the fingerprint of each line of a file, and the digest of what a cache entry rests on.

pub(crate) mod adapters;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
