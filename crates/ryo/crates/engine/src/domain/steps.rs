//! The `steps` concept: the instruction set the lowering writes, and the cost class of each instruction.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the instruction set is the contract between the lowering (track L1), which writes it, and the encoding, which reads it; until the lowering lands nothing but this crate's tests constructs a step or a stage, and the track that constructs the last one removes this"
    )
)]

pub mod calls;
pub mod lending;
pub mod service;
pub mod stages;
pub mod traversal;
pub mod types;

#[cfg(test)]
mod service_test;
