//! The `packages` concept's failures.

/// Why a manifest is refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PackagesError {
    /// The package is in a directory of a workspace that does not hold it.
    #[error("current package believes it's in a workspace when it's not")]
    NotAMember,
    /// The package takes a value from a workspace it is not in.
    #[error(
        "error inheriting a value from the workspace root manifest: failed to find a workspace root"
    )]
    NoWorkspaceToInheritFrom,
    /// The workspace cannot be read.
    #[error(transparent)]
    Workspaces(#[from] crate::domain::workspaces::errors::WorkspacesError),
}
