//! The store port, implemented with a file written as postcard writes its tables.

pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
