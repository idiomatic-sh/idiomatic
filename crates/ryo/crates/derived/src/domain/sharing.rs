//! Immutable shared positions and marks whose allocation identity never depends on liveness.
pub mod service;

#[cfg(test)]
mod service_test;

pub mod types;
