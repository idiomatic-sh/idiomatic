//! The `made_tables` concept: the tables the engine makes for a file, each on its first read and once.
#![expect(
    dead_code,
    reason = "the fill side is built in its own tree (track X, the executor split in two); the loop's tree is the caller of its seam, and the integration of the two removes this"
)]

pub mod constants;
pub mod service;
pub mod sources;
pub mod types;

#[cfg(test)]
mod service_test;
