//! Query-visible static association results, independent of the environment implementation.
pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
