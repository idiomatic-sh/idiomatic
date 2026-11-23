//! The `indexes` concept: a worker's per-file indexes, every bound column of a source as one key.

pub mod constants;
pub(crate) mod counting;
pub(crate) mod hashing;
pub mod postings;
pub mod reading;
pub mod service;
pub(crate) mod slots;
pub mod types;

#[cfg(test)]
mod service_test;
