//! The `repository` port's failures.

/// Why a question of the repository went unanswered.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The product is in no git repository.
    #[error("`{location}` is in no git repository")]
    NotARepository {
        /// Where it was asked.
        location: String,
    },
    /// A revision names no commit, or two share no ancestor.
    #[error("`{revision}` names no commit this clone holds")]
    UnknownRevision {
        /// The revision.
        revision: String,
    },
    /// Git failed.
    #[error("git failed: {reason}")]
    Failed {
        /// What it said.
        reason: String,
    },
}
