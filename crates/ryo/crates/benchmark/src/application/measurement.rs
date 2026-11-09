//! Measured run preparation, execution and decoding.
mod constants;
#[cfg(test)]
mod fixtures;
pub mod service;
/// Whether a run keeps the timing pins of its linter.
pub mod types;
#[cfg(test)]
mod service_test;
