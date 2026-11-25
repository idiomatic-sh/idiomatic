//! The `lowered` concept: a rule set as the lowering writes it, before the load-time passes and the encoding.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the lowered pack is the contract between the lowering (track L1), which writes it, the passes (track L2) and the encoding; until the lowering lands nothing but this crate's tests constructs a definition, a table or a fill, and the track that constructs the last one removes this"
    )
)]

pub mod graphs;
pub mod service;
pub mod types;

