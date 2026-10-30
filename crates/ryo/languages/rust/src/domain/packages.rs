//! The rust domain's `packages` concept: what the `Cargo.toml` files of a repository declare, read as Cargo reads them.

pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
