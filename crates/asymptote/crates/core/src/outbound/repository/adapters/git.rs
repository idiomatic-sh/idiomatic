//! The repository port, implemented with the git command line.

pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod settings;

#[cfg(test)]
mod adapter_test;
