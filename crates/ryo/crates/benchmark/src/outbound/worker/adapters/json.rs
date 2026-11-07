//! JSON request and newline-terminated response transport.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;
mod report;
mod request;
mod rules;

pub(crate) mod connection;
pub(crate) mod constants;
pub(crate) mod types;
