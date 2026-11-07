//! The `update` use case's failures.

use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `update` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
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
    /// A package could not be found.
    #[error(transparent)]
    Packages(#[from] PackagesError),
    /// No entry of that name.
    #[error("no entry `{entry}` in the root tastemaker.toml")]
    NoEntry {
        /// The entry.
        entry: String,
    },
    /// The entry's package does not offer the requested version.
    #[error("entry `{entry}` has no offered version `{version}`")]
    NoVersion {
        /// The entry.
        entry: String,
        /// The version asked for.
        version: String,
    },
    /// The prospective workspace cannot resolve its definitions or member selectors.
    #[error("cannot resolve the prospective workspace: {message}")]
    Resolution {
        /// The unresolved input.
        message: String,
    },
    /// Writing the lock, which pins the versions moved, failed.
    #[error("workspace lock could not be written: {source}; run `tm check --fix` to recover")]
    Relock {
        /// The failed lock write.
        source: RepositoryError,
    },
}
