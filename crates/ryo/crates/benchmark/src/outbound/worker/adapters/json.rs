//! JSON request and newline-terminated response transport.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;
mod schemas;

pub(crate) mod connection;
pub(crate) mod constants;
pub(crate) mod types;

