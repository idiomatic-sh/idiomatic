//! The receipts port, implemented with the local filesystem.

pub(crate) mod adapter;
mod constants;
mod invocation;
mod observation;
mod writer;

#[cfg(test)]
mod adapter_test;
