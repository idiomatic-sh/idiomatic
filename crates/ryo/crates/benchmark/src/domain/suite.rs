//! Suite shared by benchmark integrations.
/// Pure validation and transformation operations for this concept.
pub mod service;
/// Canonical values used by this domain concept.
pub mod types;

pub(crate) mod constants;

#[cfg(test)]
mod service_test;

/// Exact reference process observations; independent of any linter.
pub mod process;

/// Canonical identity for optional declared input metadata.
pub mod input_metadata;
/// Canonical identity of explicit import roots and Given entry observations.
pub mod project_inputs;

#[cfg(test)]
mod input_metadata_test;

pub(crate) mod observations;

pub mod rule_context;
