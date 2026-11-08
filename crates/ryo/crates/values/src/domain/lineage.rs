//! The `values` domain's `lineage` concept: which file the rows of a step come from, and which of its columns lie in that file.
//! The one definition of when a column states the file of its row, read by the planner's rewrites and by the engine alike.
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
