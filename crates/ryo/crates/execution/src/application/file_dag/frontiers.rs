//! Producer-validated ordinary frontier handover and ingress.
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
