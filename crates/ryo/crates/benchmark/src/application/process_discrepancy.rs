//! Require a completed numeric failure and every declared byte; no adapter error is itself evidence.
pub(crate) mod errors;
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
