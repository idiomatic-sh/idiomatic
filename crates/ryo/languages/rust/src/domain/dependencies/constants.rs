//! The names of the `dependencies` concept: how a manifest spells taking a requirement from the workspace.

/// How a dependency says it takes its requirement from the workspace's `[workspace.dependencies]`, as the Cargo Book writes it.
pub(crate) const INHERITED: &str = "workspace = true";
