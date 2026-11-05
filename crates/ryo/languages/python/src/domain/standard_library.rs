//! Python standard-library membership, independent of import sorting policy.
mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
