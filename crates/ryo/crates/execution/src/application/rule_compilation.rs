//! Cohesive bounded rule compilation, composed by the lint use case.
pub(crate) mod constants;
pub mod errors;
#[cfg(test)]
mod eviction_service_test;
pub(crate) mod identity;
mod observing;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
