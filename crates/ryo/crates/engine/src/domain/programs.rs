//! The `programs` concept: a rule set's programs and the instruction set they are written in.
#![expect(
    dead_code,
    reason = "the programs' body is read by the executor (track X) and written to a pack by the passes' track (L2); until they land a scan runs nothing and a pack holds no rule, and the track that reads the last field removes this"
)]

pub mod collections;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
