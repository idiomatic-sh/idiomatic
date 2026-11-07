//! The `create` use case's failures.

use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `create` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CreateError {
    /// The repository could not be read or written.
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
    /// The package could not be found.
    #[error(transparent)]
    Packages(#[from] PackagesError),
    /// The workspace names no package by path, and none was given.
    #[error("the workspace names no package by path: pass `--package <dir>`")]
    NoPackage,
    /// A rule or a template, and no extension named for its scenario files: Tastemaker never guesses one.
    #[error("`{kind}/{name}` needs the extension its scenario files end in")]
    NoExtension {
        /// Its kind, as written.
        kind: String,
        /// Its name.
        name: String,
    },
    /// The definition exists.
    #[error("`{file}` exists")]
    Exists {
        /// Its file.
        file: String,
    },
}
