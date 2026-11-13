//! The `core` domain's `coverage` concept: which relations a file is read in part, and what an operator that trusts the absence of
//! rows does about it.

pub(crate) mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
