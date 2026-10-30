//! The `core` domain's `tree_walk` concept: walking the parent and child hierarchy of a file from the start rows of an `axis`,
//! with what the walk takes from the scratch while it runs.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
