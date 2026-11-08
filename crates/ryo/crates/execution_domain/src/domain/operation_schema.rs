//! The `core` domain's `operation_schema` concept.
pub mod projection;
pub mod service;
pub mod types;

#[cfg(test)]
mod join_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
