//! The worker process-boundary transport.
pub(crate) mod adapters;
pub mod factory;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;

/// Shared process-boundary option encoding used by emitters, decoders and receipts.
pub(crate) mod schemas;
pub mod types;
