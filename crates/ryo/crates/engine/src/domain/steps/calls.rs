//! The `calls` part of the `steps` concept: the call contract. What a `Call` of each per-row meaning takes, what it gives
//! and in what order, how many rows it gives for one row, and so what bounds its work.

pub mod service;
pub mod types;

#[cfg(test)]
pub mod fixtures;
