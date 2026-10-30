//! The `core` domain's `globbing` concept: whether a glob matches a text.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
