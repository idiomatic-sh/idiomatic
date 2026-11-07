//! The `language` domain's `symbols` concept: the names of a file.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
