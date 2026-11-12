//! Central generation-scoped scheduling, shared graph results and last-use release.
pub(crate) mod axis_resources;
pub mod derived;
pub(crate) mod execution;
#[cfg(test)]
mod graph_resources;
pub(crate) mod invocation;
pub(crate) mod predicates;
pub(crate) mod resources;
pub mod service;
#[cfg(any(test, feature = "fixtures"))]
pub mod testing;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;

pub(crate) mod executor;

pub mod observations;
mod ordered_existence;

pub mod program;

pub mod frontiers;
