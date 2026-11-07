//! The inventory port, implemented with the local filesystem.

pub(crate) mod adapter;
mod case;
mod constants;
mod discrepancy;
mod invocation;
mod output;
mod registration;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
