//! The `core` domain's `recording` concept: what a visit of a file comes to for the store the run keeps, and how what an earlier run
//! kept of a file is used again.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
