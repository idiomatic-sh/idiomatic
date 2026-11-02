//! Central generation-scoped scheduling, shared graph results and last-use release.
pub(crate) mod aggregation;
pub(crate) mod axis_resources;
pub(crate) mod execution;
pub(crate) mod graph_resources;
pub(crate) mod invocation;
pub(crate) mod predicates;
pub(crate) mod request;
pub(crate) mod resources;
pub(crate) mod service;
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
pub(crate) mod optimized;
pub(crate) mod selected;
