//! Central generation-scoped scheduling, shared graph results and last-use release.
pub mod derived;
pub(crate) mod execution;
#[cfg(test)]
mod graph_resources;
pub(crate) mod predicates;
pub(crate) mod resources;
pub mod service;
#[cfg(test)]
mod service_test;
#[cfg(any(test, feature = "fixtures"))]
pub mod testing;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub(crate) mod executor;
pub(crate) mod fact_lookup;
pub(crate) mod file_pass;

pub mod observations;
mod ordered_existence;

pub mod program;

pub mod frontiers;
