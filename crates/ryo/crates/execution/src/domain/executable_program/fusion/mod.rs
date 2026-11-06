//! Fusion eligibility: which compiled instructions may continue their producer's region.
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
