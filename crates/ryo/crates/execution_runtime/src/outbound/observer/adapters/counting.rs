//! The observer port, implemented by counting the rows each step is fed.

pub(crate) mod adapter;
pub(crate) mod constants;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
