//! The `values` domain's `plans` concept: a compiled query, its steps and what each reads.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
