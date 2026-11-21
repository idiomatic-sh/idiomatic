//! The `made_tables` concept: the tables the engine makes for a file, each on its first read and once.
#![expect(
    dead_code,
    reason = "the fill side is built in its own tree (track X, the executor split in two); the loop's tree is the caller of its seam, and the integration of the two removes this"
)]

pub mod calling;
pub mod carrying;
pub mod constants;
pub mod fanning;
pub mod fills;
pub mod indexing;
pub mod keyed;
pub mod making;
pub mod matching;
pub mod operands;
pub mod pipelines;
pub mod reading;
pub mod service;
pub mod sources;
pub mod staging;
pub mod types;
pub mod using;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
