//! Fusion eligibility: which compiled instructions may continue their producer's region.
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
