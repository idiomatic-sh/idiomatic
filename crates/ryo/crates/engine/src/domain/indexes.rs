//! The `indexes` concept: a worker's per-file indexes, every bound column of a source as one key.
#![expect(
    dead_code,
    reason = "the loops are built and timed before the executor that runs them (track X, the reference bench first); the executor's commit calls the last of them and removes this"
)]

pub mod constants;
pub mod reading;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
