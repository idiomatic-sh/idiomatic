//! Duplicate-insensitive membership; no domain cache identity or admission policy.
pub(crate) mod service;

#[cfg(test)]
mod service_test;

pub mod types;
