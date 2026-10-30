//! The `add` use case's failures.

use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `add` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AddError {
    /// The repository could not be read or written.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// The workspace's file could not be read or edited.
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
    /// No package the workspace names holds the definition.
    #[error("no package of the workspace holds `{definition}`")]
    NoDefinition {
        /// What was asked.
        definition: String,
    },
    /// The prospective workspace cannot resolve its definitions or member selectors.
    #[error("cannot resolve the prospective workspace: {message}")]
    Resolution {
        /// The unresolved input.
        message: String,
    },
    /// The manifest is updated but writing its lock failed.
    #[error(
        "workspace manifest is updated, but its lock could not be written: {source}; run `tm check --fix` to recover"
    )]
    Relock {
        /// The failed lock write.
        source: RepositoryError,
    },
}
