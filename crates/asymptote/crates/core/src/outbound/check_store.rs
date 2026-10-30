//! The `check_store` port: the store a product keeps at `.asymptote/cache/check.store`, gitignored by itself: the
//! last check's runs, so a tool whose inputs did not move is not run again and `up` and `down` run no tool.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
