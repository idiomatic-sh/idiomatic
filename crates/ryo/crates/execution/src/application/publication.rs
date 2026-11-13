//! The `publication` application operation: producer-local names become portable values before the producer's symbols are dropped.
pub mod errors;
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
