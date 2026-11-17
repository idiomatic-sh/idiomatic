//! The `manifest` concept: the tools a product asks asymptote to run, as its `asymptote.toml` names them.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
