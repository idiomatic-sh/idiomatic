//! The `core` domain's `operating` concept: what an operator reads, writes and fails with.

pub(crate) mod errors;
#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
