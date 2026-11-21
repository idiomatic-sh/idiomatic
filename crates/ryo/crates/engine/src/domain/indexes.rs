//! The `indexes` concept: a worker's per-file indexes, every bound column of a source as one key.
#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the fill side's one caller is the loop's tree, so the library alone reaches nothing here, while the bench's fixtures reach all of it: an expectation would be unfulfilled in one build or the other; the integration of the two trees removes this"
    )
)]

pub mod constants;
pub mod reading;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
