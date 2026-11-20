//! The `digests` concept: the digest that names a plan's step by everything it reads, the same on every build, toolchain
//! and machine. The lowering's node store is its one reader.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the node digests are the contract the lowering's node store (track L1) reads; until the lowering lands nothing but this crate's tests digests a plan, and the track that calls `node_digests` removes this"
    )
)]

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
