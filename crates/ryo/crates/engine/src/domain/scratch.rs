//! The `scratch` concept: a worker's per-file state.

pub mod kept;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
