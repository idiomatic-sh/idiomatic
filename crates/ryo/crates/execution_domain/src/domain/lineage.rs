//! The `lineage` concept: which file the rows of a step come from, and which of its columns lie in that file.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub mod types;
