//! Central generation-scoped scheduling, shared graph results and last-use release.
pub mod derived;
pub(crate) mod execution;
#[cfg(test)]
mod graph_resources;
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
pub(crate) mod fact_lookup;
#[cfg_attr(not(test), expect(dead_code, reason = "the file pass is built beside the interpreter; run_file reaches it at the switch"))]
pub(crate) mod file_pass;

pub mod observations;
mod ordered_existence;

pub mod program;

pub mod frontiers;
