//! The `core` domain's `replay` concept: how the rows a step makes are passed on, one by one, to what takes them.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
