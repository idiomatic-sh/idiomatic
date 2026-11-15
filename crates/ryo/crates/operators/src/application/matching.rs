//! The `matching` application operation: the kernel's matching use case answers the domain's pattern semantics.

pub mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
