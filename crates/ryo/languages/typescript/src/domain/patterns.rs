//! The typescript domain's `patterns` concept: the regular-expression patterns a file carries, parsed by the language's library and kept
//! under the element that carries each, and what their rows read of them.

pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
