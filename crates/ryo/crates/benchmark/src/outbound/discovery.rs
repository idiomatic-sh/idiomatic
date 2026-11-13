//! The source files a pinned tool selects by default in one corpus worktree.
pub(crate) mod adapters;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
