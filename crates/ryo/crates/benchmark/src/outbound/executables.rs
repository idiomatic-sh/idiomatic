//! Builds, seals and runs the executables an operation of a linter bench needs.
pub(crate) mod adapters;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub mod factory;

pub mod types;
