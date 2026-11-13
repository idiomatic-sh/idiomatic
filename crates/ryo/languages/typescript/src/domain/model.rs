//! The typescript domain's `model` concept: what the language works out of a file once, on a reader's first lookup, for its readers.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
