//! The `list` use case's failures.

use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `list` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ListError {
    /// The repository could not be read.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// The workspace's file could not be read.
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
    /// ryo's catalog could not be read.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
    /// Nothing of the kinds asked has a name or a description that holds the term.
    #[error(
        "nothing listed holds `{term}`: it is no kind (rules, specs, templates, relations or operators) and no name"
    )]
    NoMatch {
        /// The term.
        term: String,
    },
}
