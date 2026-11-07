//! The `packages` port: where package resolution stays, behind one port (path today; git and a registry later).

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
