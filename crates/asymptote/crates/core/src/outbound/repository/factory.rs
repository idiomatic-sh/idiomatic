//! Builds the repository adapters, one function each. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::types::Worktree;
use asymptote_values::domain::manifest::types::ProductRoot;

/// The git adapter over the product at `root` and `worktree`, the working tree the edge listed once for every product in the repository, or why
/// it could not be: the git the user runs, so its rename detection is git's own.
#[must_use]
pub fn git(
    root: ProductRoot,
    worktree: Result<Worktree, RepositoryError>,
) -> impl crate::outbound::repository::interface::RepositoryPort {
    super::adapters::git::adapter::RepositoryGitAdapter::new(root, worktree)
}
