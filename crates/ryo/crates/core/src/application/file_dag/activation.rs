//! Request-owned immutable activation metadata; file data and lifetime counters stay local.
mod service;
pub(in crate::application) mod types;

#[cfg(test)]
mod service_test;
