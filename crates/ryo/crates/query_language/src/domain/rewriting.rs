//! The `query_language` domain's `rewriting` concept: the idiom of a plan that says in many rows what one aggregate says in few: the pairs of functions whose sets of texts are equal.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
