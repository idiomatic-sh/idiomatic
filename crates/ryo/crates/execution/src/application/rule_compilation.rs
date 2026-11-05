//! Cohesive bounded rule compilation, composed by the lint use case.
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod identity;
mod observing;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;

pub(crate) mod replay;
