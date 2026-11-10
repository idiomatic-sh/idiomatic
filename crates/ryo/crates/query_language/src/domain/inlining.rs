//! The `query_language` domain's `inlining` concept: the views of a relation read in place, and what an operation reads of them.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
