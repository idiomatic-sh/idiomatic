//! Builds the hooks adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The git adapter: the hooks directory is the one git itself answers for, so `core.hooksPath` and worktrees hold.
#[must_use]
pub fn git() -> impl crate::outbound::hooks::interface::HooksPort {
    super::adapters::git::adapter::HooksGitAdapter
}
