//! The normalized spelling of an invocation, shared by captures and offline replay.
mod constants;
pub(crate) mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
