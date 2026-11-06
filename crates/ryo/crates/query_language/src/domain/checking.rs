//! The `query_language` domain's `checking` concept: the checks of a module before its calls are expanded.

mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
