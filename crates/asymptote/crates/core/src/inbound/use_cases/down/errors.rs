//! The `down` use case's failures.

use crate::domain::operations::errors::OperationsError;
use crate::outbound::lock::errors::LockError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `down` drained nothing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DownError {
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
    /// A tool of the check failed, so what is resolved is unknown.
    #[error("{count} tool(s) failed in the last check, `{first}` among them")]
    ToolsFailed {
        /// The first.
        first: String,
        /// How many.
        count: usize,
    },
}

impl From<crate::application::accepted_state::errors::AcceptedStateError> for DownError {
    fn from(error: crate::application::accepted_state::errors::AcceptedStateError) -> Self {
        use crate::application::accepted_state::errors::AcceptedStateError;
        match error {
            AcceptedStateError::Lock(error) => Self::Lock(error),
            AcceptedStateError::Fold(error) => Self::Fold(error),
            AcceptedStateError::Repository(error) => Self::Repository(error),
        }
    }
}

impl From<crate::application::cached_report::errors::CachedReportError> for DownError {
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
