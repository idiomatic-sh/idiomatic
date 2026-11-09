//! The `receipts` port: where a run appends its raw case receipts and the raw process bytes kept beside them.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
