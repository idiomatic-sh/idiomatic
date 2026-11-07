//! The sarif port, implemented with `serde-sarif`.

pub(crate) mod adapter;
mod constants;
mod schemas;

#[cfg(test)]
mod adapter_test;
