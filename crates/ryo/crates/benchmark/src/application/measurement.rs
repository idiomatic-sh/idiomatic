//! Measured run preparation, execution and decoding.
mod constants;
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
/// Whether a run keeps the timing pins of its linter.
pub(crate) mod types;
