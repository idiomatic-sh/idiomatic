//! Explicit authored outputs and typed dependencies between rules; no scheduling or storage.
pub mod errors;
pub mod types;
pub mod validated;

/// Canonical declaration identity.
pub mod service;

#[cfg(test)]
mod service_test;

/// Persistent named-root readiness schema identity.
pub mod constants;
