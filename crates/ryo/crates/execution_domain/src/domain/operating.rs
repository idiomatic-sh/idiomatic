//! The `core` domain's `operating` concept: what an operator reads, writes and fails with.

pub(crate) mod constants;
pub mod errors;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
