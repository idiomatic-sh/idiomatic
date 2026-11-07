//! Producer-validated ordinary frontier handover and ingress.
pub(crate) mod constants;
pub mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
