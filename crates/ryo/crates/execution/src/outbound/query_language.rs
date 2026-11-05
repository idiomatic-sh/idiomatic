//! Required query compiler contract used during request preparation.
pub(crate) mod errors;
pub(crate) mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
