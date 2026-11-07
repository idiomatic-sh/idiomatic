//! Mandatory per-operation metadata and the finite primitive capability boundary.
pub(crate) mod axis;
pub(crate) mod context;
pub(crate) mod driver;
pub(crate) mod failure;

pub(crate) mod graph;
pub(crate) mod graph_view;
pub(crate) mod join;
pub(crate) mod lease;
pub(crate) mod preflight;
pub(crate) mod registration;
pub(crate) mod resources;
pub(crate) mod service;
pub(crate) mod types;
pub(crate) mod views;

pub(crate) mod derived;

pub(crate) mod workspace;

#[cfg(test)]
mod workspace_test;

#[cfg(test)]
mod row_population_test;

#[cfg(test)]
mod join_result_test;
