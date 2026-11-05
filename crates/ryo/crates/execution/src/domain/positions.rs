//! The `core` domain's `positions` concept: the rows of a lowered step, which are the positions of the elements of a file, and what
//! is read from them.

mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
pub(crate) mod service_test;
