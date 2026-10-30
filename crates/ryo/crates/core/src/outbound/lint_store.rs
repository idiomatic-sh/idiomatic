//! The `lint_store` port: what a lint keeps for the next run, read whole at the start and kept once when the run succeeds.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
