//! The `core` domain's `comparing` concept: whether the comparisons of a select hold for a row, decided by the family of each test
//! and remembered by name and by text.

pub mod collections;
pub mod service;
pub mod types;

pub mod errors;
#[cfg(test)]
mod service_test;
