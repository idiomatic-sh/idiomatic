//! The `core` domain's `operator` concept.

pub mod constants;
pub mod contract;
pub mod service;
pub mod types;

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod service_test;
