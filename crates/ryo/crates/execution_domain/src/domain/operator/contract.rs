//! Mandatory per-operation metadata and the finite primitive capability boundary.
pub mod context;
pub(crate) mod driver;
pub mod failure;
pub mod presence;

pub mod graph_view;
pub mod lease;
pub(crate) mod preflight;
pub mod primitive;
pub mod registration;
pub mod resources;
pub mod service;
pub mod types;
pub mod views;

pub mod workspace;

#[cfg(test)]
mod service_test;
