//! The json domain's `model` concept: what is worked out of a file once, when it is parsed, and given to every reader of it.

mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
