//! One prepared shared ordering and one reusable partition-local adjacency.
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
