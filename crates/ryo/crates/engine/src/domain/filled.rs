//! The `filled` concept: the relations a language filled for a file, each unfilled, filled for some keys, or whole.
#![expect(
    dead_code,
    reason = "the fill side is built in its own tree (track X, the executor split in two); the loop's tree is the caller of its seam, and the integration of the two removes this"
)]

pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
