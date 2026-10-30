//! The manifest port, implemented with `toml` and `toml_edit`.

pub(crate) mod adapter;
mod constants;
mod schemas;

#[cfg(test)]
mod adapter_test;
