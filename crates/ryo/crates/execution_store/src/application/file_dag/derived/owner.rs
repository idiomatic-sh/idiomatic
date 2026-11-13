//! Central acquisition, mandatory working ownership and optional retained capacity.
pub mod acquisition;
mod backings;
pub mod cache;
pub mod cached_graph;
mod diagnostics;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
mod graph;
mod graph_key;
mod graph_policy;
mod graph_retention;
pub mod graphs;
pub mod index;
mod leases;
mod lifetime;
pub mod service;
#[cfg(test)]
mod service_test;
mod setup;
pub mod types;
#[cfg(any(test, feature = "fixtures"))]
pub mod witness;
