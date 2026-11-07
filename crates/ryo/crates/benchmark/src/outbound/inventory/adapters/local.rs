//! The inventory port, implemented with the local filesystem.

pub(crate) mod adapter;
mod constants;
mod schemas;

#[cfg(test)]
mod adapter_test;
