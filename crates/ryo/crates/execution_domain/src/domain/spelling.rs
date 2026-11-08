//! The `core` domain's `spelling` concept: whether names that share a hash are spelled the same.

pub(crate) mod errors;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
