//! The `indexes` concept: a worker's per-file indexes, every bound column of a source as one key.

pub mod constants;
pub mod reading;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
