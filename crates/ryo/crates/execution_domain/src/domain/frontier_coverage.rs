//! The `frontier_coverage` concept: what a coverage says of its files is carried to the file-frontier boundary and restored from it.
pub(crate) mod constants;
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
