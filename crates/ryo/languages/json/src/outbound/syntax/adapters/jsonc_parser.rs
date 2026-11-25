//! The syntax port, implemented with `jsonc_parser`.

pub(crate) mod adapter;
mod constants;
mod interface;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod schemas;
pub(crate) mod types;
mod walk;

#[cfg(test)]
mod adapter_test;
