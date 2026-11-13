//! The context port, implemented with the local filesystem.

pub(crate) mod adapter;
mod constants;
mod schemas;
mod writer;

#[cfg(test)]
mod adapter_test;
