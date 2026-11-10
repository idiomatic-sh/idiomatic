//! The `core` domain's `spelling` concept: whether names that share a hash are spelled the same.

pub mod errors;
pub mod service;

#[cfg(test)]
mod service_test;
