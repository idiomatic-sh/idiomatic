//! The `values` domain's `plans` concept: the steps of a compiled query, the operations they are and what each reads.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
