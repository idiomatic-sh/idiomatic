//! The `receipts` port: where a run appends its raw case receipts and the raw process bytes kept beside them.

pub mod errors;
pub mod interface;
pub mod types;
pub(crate) mod adapters;
pub mod factory;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
