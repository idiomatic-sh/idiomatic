//! Central generation-scoped scheduling, shared graph results and last-use release.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
