//! Lazy decoded brace-format parsing, separate from percent-format parsing.
pub(crate) mod interface;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
