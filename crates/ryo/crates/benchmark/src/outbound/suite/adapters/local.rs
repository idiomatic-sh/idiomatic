//! Local boundary implementation.
pub(crate) mod adapter;
mod constants;
mod schemas;

#[cfg(test)]
mod adapter_test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
