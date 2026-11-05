//! Central generation-scoped scheduling, shared graph results and last-use release.
pub(crate) mod aggregation;
pub(crate) mod axis_resources;
pub(crate) mod execution;
#[cfg(test)]
mod graph_resources;
pub(crate) mod invocation;
pub(crate) mod predicates;
pub(crate) mod request;
pub(crate) mod resources;
#[cfg(test)]
mod service_test;
#[cfg(test)]
pub(crate) mod testing;
pub(crate) mod types;
pub(crate) mod workspace;

#[cfg(test)]
mod contract_route_test;

pub(crate) mod cached_graph;
pub(crate) mod closure_partitions;
#[cfg(test)]
mod contract_timing_test;
pub(crate) mod selected;

pub(in crate::application) mod relations;

pub(crate) mod executor;
pub(crate) mod pipeline;

#[cfg(test)]
pub(crate) mod acceptance;


pub(crate) mod derived;

pub(crate) mod route_counts;

mod equality_join;

pub(crate) mod observations;
mod ordered_existence;

pub(in crate::application) mod completion;
pub(crate) mod scope;

pub(crate) mod program;

pub(crate) mod frontiers;
