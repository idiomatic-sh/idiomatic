//! Owner-only producer name normalization and publication storage.
pub(crate) mod constants;
pub mod publication;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
