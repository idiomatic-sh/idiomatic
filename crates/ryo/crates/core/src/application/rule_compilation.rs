//! Cohesive bounded rule compilation, composed by the lint use case.
pub(crate) mod constants;
pub(crate) mod errors;
mod observing;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
