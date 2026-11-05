//! The `core` domain's `table_model` concept.

pub(crate) mod collections;
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

pub(crate) mod reading;
