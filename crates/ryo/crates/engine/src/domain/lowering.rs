//! The `lowering` concept: each selected rule's plan turned into its programs, or the plan shape the lowering cannot express named.

pub mod constants;
pub(crate) mod costing;
pub(crate) mod definitions;
pub(crate) mod emptiness;
pub mod errors;
pub(crate) mod flattening;
pub(crate) mod nodes;
pub(crate) mod placement;
pub(crate) mod planning;
pub(crate) mod proving;
pub mod service;
pub(crate) mod tables;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
