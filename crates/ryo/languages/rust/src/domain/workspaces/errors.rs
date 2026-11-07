//! The `workspaces` concept's failures.

use super::types::Reason;
use crate::domain::paths::types::RepoPath;

/// Why the packages of a workspace cannot be told.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WorkspacesError {
    /// A manifest the workspace names cannot be read.
    #[error("failed to load manifest for workspace member `{manifest}`: {reason}")]
    Member {
        /// The manifest.
        manifest: RepoPath,
        /// What the reader said.
        reason: Reason,
    },
    /// A manifest cannot be read.
    #[error("failed to read `{manifest}`: {reason}")]
    Unreadable {
        /// The manifest.
        manifest: RepoPath,
        /// What the reader said.
        reason: Reason,
    },
}
