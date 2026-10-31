//! WorkspaceGitAdapter-backed corpus cache.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;

mod constants;

pub(crate) mod types;

pub(crate) mod settings;
