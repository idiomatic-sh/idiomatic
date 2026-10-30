//! The `lock` port: the immutable, content-addressed operations under `.asymptote/operations`, kept byte for byte as
//! one canonical text, so concurrent branches add different files and merge without a conflict.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
