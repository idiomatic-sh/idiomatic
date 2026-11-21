//! The `statistics` concept: what a sample run measured of a rule set's definitions, and what the passes decide from it.
#![expect(
    dead_code,
    reason = "the statistics are the contract between the passes' track (L2), which reads and writes their bytes, the lowering (L1), which reads them, and the executor (X), which counts; until they land every load runs with none, and the track that reads the last of them removes this"
)]

pub(crate) mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
