//! Graph memo behavior remains under the generation-owned file cache.

pub(crate) mod roots;

/// Graph-priority reclamation with live ownership checks.
pub(crate) mod reclamation;

pub(in crate::application::file_dag::derived) mod footprint;
