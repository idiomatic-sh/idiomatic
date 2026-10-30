//! The rust domain's `dependencies` concept: what a package depends on by path, as Cargo's metadata says it.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
