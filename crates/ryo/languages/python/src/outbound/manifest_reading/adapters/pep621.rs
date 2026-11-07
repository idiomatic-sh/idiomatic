//! The manifest reading port, implemented for PEP 621 and PEP 735 in `pyproject.toml`.

pub(crate) mod adapter;
pub(crate) mod schemas;

#[cfg(test)]
mod adapter_test;
