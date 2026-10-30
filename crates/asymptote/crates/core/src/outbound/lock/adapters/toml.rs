//! The lock port, implemented with `toml` and `blake3`, byte for byte in one canonical text.

pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod settings;

#[cfg(test)]
mod adapter_test;
