//! Shared diagnostics vocabulary.
pub mod errors;
pub mod types;

/// Pure invariants shared by report producers, decoders and comparisons.
pub mod service;

#[cfg(test)]
mod service_test;

pub mod constants;
