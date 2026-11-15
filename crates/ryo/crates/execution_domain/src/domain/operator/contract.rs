//! Mandatory per-operation metadata and the finite primitive capability boundary.
pub mod failure;
pub mod presence;

pub(crate) mod preflight;
pub mod registration;
pub mod resources;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
