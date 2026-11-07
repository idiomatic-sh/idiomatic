//! The syntax port, implemented with `cssparser`.

pub(crate) mod adapter;
pub(crate) mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod schemas;
pub(crate) mod types;
mod writer;

#[cfg(test)]
mod adapter_test;
