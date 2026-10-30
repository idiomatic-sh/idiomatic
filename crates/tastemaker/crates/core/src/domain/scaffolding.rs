//! The `scaffolding` concept: what a file an entry places is named and rendered with when it is first written.

pub(crate) mod constants;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
