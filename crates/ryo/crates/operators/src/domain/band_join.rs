//! Band joins: each left row with the right rows of its key inside a band of order comparisons across the sides, every one or
//! the nearest.
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
