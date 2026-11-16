//! Bencher Metric Format, consumed by Asymptote.
pub(crate) mod adapter;
pub(crate) mod connection;
mod document;
mod schemas;

#[cfg(test)]
mod adapter_test;

mod constants;

pub(crate) mod types;
