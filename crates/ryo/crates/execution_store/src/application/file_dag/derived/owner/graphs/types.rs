//! Graph memo behavior remains under the generation-owned file cache.

pub mod roots;

/// Graph-priority reclamation with live ownership checks.
pub(crate) mod reclamation;

pub(crate) mod reserve;

pub(crate) mod retained;

pub mod handles;
