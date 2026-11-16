//! Suite shared by benchmark integrations.
/// Pure validation and transformation operations for this concept.
pub mod service;
/// Canonical values used by this domain concept.
pub mod types;

pub(crate) mod constants;
/// How long one case's whole run may take to verify, for every linter.
pub use constants::CASE_TIME_LIMIT;

#[cfg(test)]
mod service_test;
