//! Central generation-scoped scheduling, shared graph results and last-use release.
pub(crate) mod axis_resources;
pub(crate) mod execution;
#[cfg(test)]
mod graph_resources;
pub(crate) mod invocation;
pub(crate) mod predicates;
pub mod service;
pub(crate) mod resources;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod testing;

#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod workspace;

#[cfg(test)]
mod service_test;

pub(crate) mod closure_partitions;
pub(crate) mod selected;

pub(crate) mod relations;

pub(crate) mod executor;

#[cfg(test)]
pub(crate) mod acceptance;

pub(crate) mod derived;

pub mod route_counts;

pub mod observations;
mod ordered_existence;

pub(crate) mod scope;

pub mod program;

pub mod frontiers;
