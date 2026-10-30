//! The manifest reading port, implemented for the tables Poetry reads in `pyproject.toml`.

pub(crate) mod adapter;
pub(crate) mod schemas;

#[cfg(test)]
mod adapter_test;
