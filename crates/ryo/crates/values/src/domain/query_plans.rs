//! The `values` domain's `query_plans` concept: a compiled query whole, what it emits and the relations it reads.

pub mod digesting;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
