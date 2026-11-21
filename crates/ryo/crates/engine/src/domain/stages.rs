//! The `stages` concept: the compiled loops a fill's pipeline runs, one per kind of stage.
#![expect(
    dead_code,
    reason = "the loops are built and timed before the executor that runs them (track X, the reference bench first); the executor's commit calls the last of them and removes this"
)]

pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
