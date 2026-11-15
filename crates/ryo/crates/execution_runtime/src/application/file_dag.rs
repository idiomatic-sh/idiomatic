//! Central generation-scoped scheduling, shared graph results and last-use release.
pub mod derived;
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
