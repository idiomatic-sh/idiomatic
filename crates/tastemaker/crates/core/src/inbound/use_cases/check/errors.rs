//! The `check` use case's failures.

use crate::domain::placement::errors::PlacementError;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `check` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    /// Independent work could not be scheduled.
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
    /// The selected definition has no current successful validation proof.
    #[error("`{subject}` has no current successful proof ({reason}); run `tm validate`")]
    Proof {
        /// The definition version.
        subject: String,
        /// Whether the proof is absent, stale or failed.
        reason: String,
    },
    /// The repository could not be read.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// A definition could not be read.
    #[error("`{file}`: {source}")]
    Manifest {
        /// The file.
        file: String,
        /// Why.
        source: ManifestError,
    },
    /// A package could not be found or read.
    #[error(transparent)]
    Packages(#[from] PackagesError),
    /// The nearest `tastemaker.toml` is a local one.
    #[error(
        "`{file}` is a local tastemaker.toml: run from the workspace whose root file names it in `members`"
    )]
    NotARoot {
        /// The file.
        file: String,
    },
    /// A release no package holds, or no dependency resolves.
    #[error("{message}")]
    Resolution {
        /// What is wrong.
        message: String,
    },
    /// The entries could not be placed.
    #[error(transparent)]
    Placement(#[from] PlacementError),
    /// The lint call failed.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
    /// A path the check is narrowed to holds no file of the workspace.
    #[error("`{path}` holds no file of the workspace")]
    Within {
        /// The path, from the repository's root.
        path: String,
    },
    /// The lock is stale or missing.
    #[error("`{file}` is {state}: run `tm check --fix` to rewrite it")]
    Lock {
        /// The lock file.
        file: String,
        /// `stale` or `missing`.
        state: String,
    },
}

impl From<crate::application::reporting::errors::ReportingError> for CheckError {
    fn from(error: crate::application::reporting::errors::ReportingError) -> Self {
        match error {
            crate::application::reporting::errors::ReportingError::Scheduler(error) => {
                Self::Scheduler(error)
            }
        }
    }
}
