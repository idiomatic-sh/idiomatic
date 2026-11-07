//! The notes port, implemented with `postcard`.

pub(crate) mod adapter;
pub(crate) mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
