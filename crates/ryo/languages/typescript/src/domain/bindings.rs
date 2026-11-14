//! The typescript domain's `bindings` concept: the declarations the parser's semantic pass resolves each reference to, and the names
//! the language provides without one.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
