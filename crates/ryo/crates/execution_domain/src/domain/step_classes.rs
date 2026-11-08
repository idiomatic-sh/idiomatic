//! Exact source/predicate equivalence of Select and pure Project steps. Output
//! projections retain independent attribution while sharing source positions.

pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
