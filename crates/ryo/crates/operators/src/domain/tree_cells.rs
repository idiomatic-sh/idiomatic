//! The `core` domain's `tree_cells` concept: the rows of a file's tree read cell by cell from its elements, never decoded.

mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
