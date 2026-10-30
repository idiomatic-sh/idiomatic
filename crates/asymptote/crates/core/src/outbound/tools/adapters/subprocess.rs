//! The tools port, implemented with `std::process` and `process-wrap`.

pub(crate) mod adapter;
pub(crate) mod connection;
mod constants;

#[cfg(test)]
mod adapter_test;
