//! The manifest port, implemented with `toml`.

pub(crate) mod adapter;
mod constants;
mod dependencies;
pub(crate) mod schemas;

#[cfg(test)]
mod adapter_test;
