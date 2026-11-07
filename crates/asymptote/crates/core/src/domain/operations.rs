//! The `operations` concept: the immutable lock operations that record accepted violations, and their fold.

pub mod constants;
pub mod errors;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
