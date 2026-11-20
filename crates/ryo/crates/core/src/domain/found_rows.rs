//! The `found_rows` concept: the rows the new engine's scan found in a file as the cells of the answer.
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
