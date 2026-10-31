//! Diagnostics shared by benchmark integrations.
/// Failures that prevent benchmark evidence from being published.
pub mod errors;
/// Pure comparison of canonical evidence.
pub mod service;
/// Canonical values used by this domain concept.
pub mod types;

#[cfg(test)]
mod service_test;

pub(crate) mod constants;
