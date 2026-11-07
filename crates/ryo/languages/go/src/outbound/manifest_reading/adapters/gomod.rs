//! The manifest reading port, implemented with the line grammar of `go.mod` and `go.work`.

pub(crate) mod adapter;
pub(crate) mod constants;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
