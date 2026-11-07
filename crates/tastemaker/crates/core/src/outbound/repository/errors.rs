//! The `repository` port's failures.

/// Why the repository could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// No `tastemaker.toml` at or above the start.
    #[error("no tastemaker.toml at or above `{start}`")]
    NoWorkspace {
        /// Where the search started.
        start: String,
    },
    /// No `.git` at or above the workspace.
    #[error("`{workspace}` is in no git repository")]
    NoRepository {
        /// The workspace's directory.
        workspace: String,
    },
    /// A path could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Read {
        /// The path.
        path: String,
        /// What the system said.
        reason: String,
    },
    /// A path could not be written.
    #[error("cannot write `{path}`: {reason}")]
    Write {
        /// The path.
        path: String,
        /// What the system said.
        reason: String,
    },
}
