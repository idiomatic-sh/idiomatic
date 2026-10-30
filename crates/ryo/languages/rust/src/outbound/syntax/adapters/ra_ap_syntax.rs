//! The syntax port, implemented with `ra_ap_syntax`.

pub(crate) mod adapter;
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
