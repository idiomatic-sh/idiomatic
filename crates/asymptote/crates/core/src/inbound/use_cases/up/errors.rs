//! The `up` use case's failures.

use crate::domain::operations::errors::OperationsError;
use crate::domain::selection::types::At;
use crate::domain::violations::types::Violation;
use crate::outbound::lock::errors::LockError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `up` recorded nothing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpError {
    /// The product's `asymptote.toml` could not be found.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The lock could not be read or written.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The lock's operations do not fold.
    #[error(transparent)]
    Fold(#[from] OperationsError),
    /// Git could not answer.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// A `--path` is no glob.
    #[error(transparent)]
    Matching(#[from] MatchingError),
    /// No check has run with this build.
    #[error("no check has run here with this asymptote")]
    NoReport,
    /// The working tree moved to another commit since the check.
    #[error("the working tree is on another commit than the last check ran on")]
    HeadMoved,
    /// The `asymptote.toml` the check ran under changed since.
    #[error("asymptote.toml changed since the last check")]
    ManifestMoved,
    /// A file the check's judgement rests on changed since.
    #[error("{count} file(s) changed since the last check, `{first}` among them")]
    Moved {
        /// The first.
        first: String,
        /// How many.
        count: usize,
    },
    /// A tool of the check failed, so what is new is unknown.
    #[error("{count} tool(s) failed in the last check, `{first}` among them")]
    ToolsFailed {
        /// The first.
        first: String,
        /// How many.
        count: usize,
    },
    /// An `--at` names a place no new violation is at.
    #[error("no new finding is at `{}:{}`", at.path.0, at.line.0)]
    NothingAt {
        /// The place.
        at: At,
        /// The new violations nearest it.
        nearest: Vec<Violation>,
    },
    /// A violation chosen lies outside the product: an absolute path, or one that climbs out of its root.
    #[error("cannot accept a finding outside the workspace: {path}")]
    Outside {
        /// Its path.
        path: String,
    },
    /// New violations have no fingerprint, so no record can accept them.
    #[error("{count} new finding(s) have no fingerprint: their file or line could not be read")]
    Unkeyed {
        /// How many.
        count: u32,
    },
}

impl From<crate::application::accepted_state::errors::AcceptedStateError> for UpError {
    fn from(error: crate::application::accepted_state::errors::AcceptedStateError) -> Self {
        use crate::application::accepted_state::errors::AcceptedStateError;
        match error {
            AcceptedStateError::Lock(error) => Self::Lock(error),
            AcceptedStateError::Fold(error) => Self::Fold(error),
            AcceptedStateError::Repository(error) => Self::Repository(error),
        }
    }
}

impl From<crate::application::cached_report::errors::CachedReportError> for UpError {
    fn from(error: crate::application::cached_report::errors::CachedReportError) -> Self {
        use crate::application::cached_report::errors::CachedReportError;
        match error {
            CachedReportError::NoReport => Self::NoReport,
            CachedReportError::HeadMoved => Self::HeadMoved,
            CachedReportError::ManifestMoved => Self::ManifestMoved,
            CachedReportError::Moved { first, count } => Self::Moved { first, count },
            CachedReportError::ToolsFailed { first, count } => Self::ToolsFailed { first, count },
        }
    }
}
