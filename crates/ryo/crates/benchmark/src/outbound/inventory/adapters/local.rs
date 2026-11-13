//! The inventory port, implemented with the local filesystem.

pub(crate) mod adapter;
mod authored;
mod case;
mod constants;
mod discrepancy;
mod invocation;
mod instance;
mod output;
mod profile;
mod registration;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
