//! The store port, implemented with a file written as postcard writes its tables.

pub(crate) mod adapter;
pub(crate) mod connection;
mod constants;
mod schemas;
pub(crate) mod types;
pub(crate) mod writer;

#[cfg(test)]
mod adapter_test;
