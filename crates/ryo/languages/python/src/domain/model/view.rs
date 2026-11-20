//! The per-file view of the semantic model: what the semantic fillers read, and where the model is built on first need.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
