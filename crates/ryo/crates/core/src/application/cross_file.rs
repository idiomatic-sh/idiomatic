//! The `cross_file` application operation.
mod coordinator;
mod inputs;
pub(crate) mod publication;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;

pub(crate) mod errors;
