//! The `embedded` port: the javascript a file of another format carries in blocks.

pub(crate) mod adapters;
pub(crate) mod factory;
pub(crate) mod interface;
pub(crate) mod types;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
