//! A request's program prepared once and run over its files: the file pass, the project phase and the publications they
//! share.
pub mod service;
#[cfg(test)]
mod service_test;
#[cfg(any(test, feature = "fixtures"))]
pub mod testing;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub(crate) mod fact_lookup;
pub(crate) mod file_pass;
pub(crate) mod project_pass;
pub(crate) mod project_source;
pub(crate) mod publications;

pub mod program;

pub mod frontiers;
