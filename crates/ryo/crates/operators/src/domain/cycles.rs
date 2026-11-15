//! A cycle search: its parts and lowering; the `components` kernel runs Tarjan's search.
pub(crate) mod service;

pub mod types;

#[cfg(test)]
mod service_test;
