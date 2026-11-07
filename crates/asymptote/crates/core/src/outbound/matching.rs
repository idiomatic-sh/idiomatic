//! The `matching` port: whether a path matches the globs `up --path` names.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
