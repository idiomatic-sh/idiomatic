//! The syntax port, implemented with `oxc_parser`.

pub(crate) mod adapter;
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod patterns;
mod projection;
mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
