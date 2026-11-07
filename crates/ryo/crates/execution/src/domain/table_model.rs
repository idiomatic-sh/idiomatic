//! The `core` domain's `table_model` concept.

pub(crate) mod collections;
mod constants;
#[cfg(test)]
pub(crate) mod fixtures;

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
