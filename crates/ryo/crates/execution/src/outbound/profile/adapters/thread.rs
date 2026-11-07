//! The profile port, implemented with a profile for each thread.

pub(crate) mod adapter;
mod constants;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
