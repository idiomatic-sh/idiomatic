//! The `core` domain's `operating` concept: what an operator reads, writes and fails with.

pub mod errors;
#[cfg(test)]
pub(crate) mod fixtures;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
