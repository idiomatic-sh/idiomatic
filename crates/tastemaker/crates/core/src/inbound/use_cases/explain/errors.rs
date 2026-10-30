//! The `explain` use case's failures.

use crate::domain::placement::errors::PlacementError;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `explain` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ExplainError {
    /// The repository could not be read or written.
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
    /// The entries could not be placed.
    #[error(transparent)]
    Placement(#[from] PlacementError),
    /// ryo's catalog could not be read.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
    /// Nothing of that name: no path, no definition, no topic.
    #[error("`{name}` is no path, no `<kind>/<name>` and no topic")]
    Unknown {
        /// What was asked.
        name: String,
    },
}
