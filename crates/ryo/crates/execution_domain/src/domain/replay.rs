//! The `core` domain's `replay` concept: how the rows a step makes are passed on, one by one, to what takes them.

pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
