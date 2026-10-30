//! The manifest reading port, implemented for the requirements files pip reads.

pub(crate) mod adapter;
pub(crate) mod constants;
pub(crate) mod schemas;

#[cfg(test)]
mod adapter_test;
