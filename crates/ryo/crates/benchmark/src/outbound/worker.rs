//! The worker process-boundary transport.
pub(crate) mod adapters;
pub mod factory;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
pub mod types;
