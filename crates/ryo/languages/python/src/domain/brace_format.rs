//! Lazy decoded brace-format parsing, separate from percent-format parsing.
pub(crate) mod service;

#[cfg(test)]
mod service_test;
pub(crate) mod types;

pub(crate) mod constants;
