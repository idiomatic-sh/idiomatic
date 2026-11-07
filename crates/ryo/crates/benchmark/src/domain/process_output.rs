//! Byte-exact process output substitution and receipt identity framing.
mod constants;
pub(crate) mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
