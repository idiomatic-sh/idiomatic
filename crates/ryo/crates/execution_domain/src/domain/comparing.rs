//! The `core` domain's `comparing` concept: whether the comparisons of a select hold for a row, decided by the family of each test
//! and remembered by name and by text.

pub(crate) mod collections;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
