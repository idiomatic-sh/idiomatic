//! The `core` domain's `table_words` concept.

pub mod constants;
pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
