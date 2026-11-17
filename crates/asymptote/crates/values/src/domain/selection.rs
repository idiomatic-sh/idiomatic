//! The `selection` concept: which new violations `up` records, by tool, rule, path pattern and location.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
