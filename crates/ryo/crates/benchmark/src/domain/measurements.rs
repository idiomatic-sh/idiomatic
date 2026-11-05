//! Measurements shared by benchmark integrations.
/// Canonical values used by this domain concept.
pub mod types;

/// Validate complete benchmark inventory identities before any timed execution.
pub mod service;

#[cfg(test)]
mod service_test;
