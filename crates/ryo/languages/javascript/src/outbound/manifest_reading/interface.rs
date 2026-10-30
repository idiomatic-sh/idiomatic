//! The manifest reading port reads manifest text into repository package declarations. An adapter says which manifest files it claims.

use super::errors::ManifestReadingError;
use crate::domain::packages::types::{Declared, ManifestPath};
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// Reads one kind of manifest into what it declares about the repository's packages.
pub(crate) trait ManifestReadingPort: Send + Sync {
    /// The names of the manifest files the adapter reads.
    fn claims(&self) -> &'static [ManifestName];

    /// The package manifest the facts of `manifest` are about: the manifest itself, unless it describes the package beside it.
    fn about(&self, manifest: &ManifestFile) -> ManifestPath;

    /// What `manifest` declares; `candidates` are the repository's files, which member globs select from.
    fn declared(
        &self,
        manifest: &ManifestFile,
        candidates: &[FilePath],
    ) -> Result<Declared, ManifestReadingError>;
}
