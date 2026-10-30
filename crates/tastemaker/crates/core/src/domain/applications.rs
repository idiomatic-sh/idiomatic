//! The `applications` concept: a placement turned into the lint calls of a check.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
