//! What the `manifest_reading` port asks of the outside: what the text of a manifest declares.

use crate::domain::manifest_validation::types::MetadataDemand;

use super::types::Claim;
use super::types::ManifestRead;
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// What every `project` adapter implements: one package manager's manifests.
pub(crate) trait ManifestReadingPort: Send + Sync {
    /// The names of the files the adapter reads as manifests, which the language asks the repository for.
    fn manifests(&self) -> &'static [ManifestName];

    /// Whether the adapter reads the file at `path`: by a name in `manifests`, or by a shape of path the names cannot say.
    fn claims(&self, path: &FilePath) -> Claim;

    /// What the claimed `manifest` declares: the package it names, the patterns of the members of a workspace it holds, and the
    /// requirements it makes. A manifest that includes another reads it from the manifests of the repository the adapter was built over.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the text is not a manifest the tool can read.
    fn read(&self, manifest: &ManifestFile, demand: MetadataDemand) -> ManifestRead;
}
