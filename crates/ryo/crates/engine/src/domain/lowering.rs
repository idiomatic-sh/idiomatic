//! The `lowering` concept: each selected rule's plan turned into its programs, or the plan shape the lowering cannot express named.

pub mod errors;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
