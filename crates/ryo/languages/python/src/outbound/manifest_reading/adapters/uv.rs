//! The manifest reading port, implemented for `uv`'s tables in `pyproject.toml`.

pub(crate) mod adapter;
pub(crate) mod schemas;

#[cfg(test)]
mod adapter_test;
