//! The `packs` concept: the byte form of a rule set's programs, for the host's store.

pub mod constants;
pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
