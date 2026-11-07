//! The `core` domain's `operating` concept: what an operator reads, writes and fails with.

pub(crate) mod errors;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
