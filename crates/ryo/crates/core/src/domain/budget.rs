//! The `core` domain's `budget` concept: how much memory a run may use, and whether the repository and the process stay within it.

pub(crate) mod errors;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
