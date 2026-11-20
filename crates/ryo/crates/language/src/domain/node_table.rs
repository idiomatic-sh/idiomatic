//! The `language` domain's `node_table` concept: one file's node table in the engine's id space, the calls a walk writes it
//! through and the readers of it.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
