//! The `core` domain's `operation_schema` concept.
pub mod projection;
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod join_test;

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod service_test;
