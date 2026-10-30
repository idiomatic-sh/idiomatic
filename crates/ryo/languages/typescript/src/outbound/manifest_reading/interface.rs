//! The manifest reading port reads manifest text into repository package declarations. An adapter says which manifest files it claims; one
//! that selects among the repository's manifests is built for each run, over that run's.

use super::errors::ManifestReadingError;
use crate::domain::packages::types::{Declared, ManifestPath};
use ryo::languages::{ManifestFile, ManifestName};

/// Reads one kind of manifest into what it declares about the repository's packages.
pub(crate) trait ManifestReadingPort: Send + Sync {
    /// The names of the manifest files the adapter reads.
    fn claims(&self) -> &'static [ManifestName];

    /// The package manifest the facts of `manifest` are about: the manifest itself, unless it describes the package beside it.
    fn about(&self, manifest: &ManifestFile) -> ManifestPath;

    /// What `manifest` declares: member globs select from the manifests of the run the adapter was built for.
    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError>;
}
