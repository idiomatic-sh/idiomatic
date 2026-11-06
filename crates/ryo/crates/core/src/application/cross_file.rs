//! The `cross_file` application operation.
mod coordinator;
mod inputs;
pub(crate) mod publication;
#[cfg(test)]
mod publication_test;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;

pub(crate) mod errors;

pub(crate) mod preparation;
