//! The repository port, implemented over the disk.

pub(crate) mod adapter;
mod constants;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
