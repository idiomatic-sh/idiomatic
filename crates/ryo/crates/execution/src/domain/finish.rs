//! Owner-only producer name normalization and publication storage.
pub(crate) mod errors;
pub(crate) mod publication;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
