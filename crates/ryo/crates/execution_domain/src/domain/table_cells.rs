//! The `core` domain's `table_cells` concept.

pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
#[cfg(test)]
pub(crate) mod service_test;
