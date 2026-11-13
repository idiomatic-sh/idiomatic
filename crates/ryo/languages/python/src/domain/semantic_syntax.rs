//! Explicit projection of native syntax and embedded annotation expressions.
pub(crate) mod service;

#[cfg(test)]
mod service_test;

pub(crate) mod types;

pub(crate) mod constants;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;

mod generic_subscription;
