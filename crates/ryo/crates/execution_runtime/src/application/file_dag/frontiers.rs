//! Producer-validated ordinary frontier handover and ingress.
pub(crate) mod cache;
pub(crate) mod constants;
pub(crate) mod decodes;
pub mod errors;
#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
