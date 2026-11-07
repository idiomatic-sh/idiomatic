//! Shared diagnostics vocabulary.
pub mod errors;
pub mod types;

#[cfg(test)]
mod errors_test;

/// Pure invariants shared by report producers, decoders and comparisons.
pub mod service;

#[cfg(test)]
mod service_test;

mod constants;
