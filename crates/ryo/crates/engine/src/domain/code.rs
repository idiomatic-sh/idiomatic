//! The `code` concept: a rule set's programs as the executor runs them and the pack stores them, and the one place a word, a
//! pool record and a small code are written and read.
#![expect(
    dead_code,
    reason = "the code is the contract between the encoding, which writes each word through its constructor, and the executor (track X), which reads it through its reader, with the specialised codes the passes' track (L2) writes; until they land nothing but this crate's tests reads a word, and the track that reads the last one removes this"
)]

pub mod anchoring;
pub mod constants;
pub mod controlling;
pub mod deciding;
pub mod filling;
pub mod records;
pub mod service;
pub mod small_codes;
pub mod types;
pub mod walking;

#[cfg(test)]
mod service_test;
