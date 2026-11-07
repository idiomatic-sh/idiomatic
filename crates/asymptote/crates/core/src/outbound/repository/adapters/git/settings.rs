//! What the git repository adapter is configured with.

use crate::domain::manifest::types::ProductRoot;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::types::Worktree;

/// The git repository adapter's settings: the values and parts its
/// composition chooses.
#[derive(Debug)]
pub(crate) struct RepositoryGitSettings {
    /// The product every question is asked of.
    pub root: ProductRoot,
    /// The repository's working tree, listed once by the edge for every product in it, or why it could not be.
    pub worktree: Result<Worktree, RepositoryError>,
}
