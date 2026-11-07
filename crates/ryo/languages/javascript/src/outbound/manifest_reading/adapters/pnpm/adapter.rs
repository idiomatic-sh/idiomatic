//! The packages and catalogs a pnpm workspace declares in YAML, about the package manifest beside it.

use super::constants::MANIFEST;
use super::schemas::read;
use crate::domain::packages::constants::PACKAGE;
use crate::domain::packages::service::{directory, relative};
use crate::domain::packages::types::{Declared, ManifestPath, RepositoryPath};
use crate::outbound::manifest_reading::{
    errors::ManifestReadingError, interface::ManifestReadingPort,
};
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// The manifest file a pnpm workspace is declared in.
const CLAIMS: &[ManifestName] = &[ManifestName(MANIFEST)];

/// Reads the members and catalogs of a pnpm workspace.
#[derive(Debug)]
pub(crate) struct ManifestReadingPnpmAdapter;

impl ManifestReadingPort for ManifestReadingPnpmAdapter {
    fn claims(&self) -> &'static [ManifestName] {
        CLAIMS
    }

    fn about(&self, manifest: &ManifestFile) -> ManifestPath {
        relative(
            directory(RepositoryPath(&manifest.path.0)),
            RepositoryPath(PACKAGE),
        )
    }

    fn declared(
        &self,
        manifest: &ManifestFile,
        candidates: &[FilePath],
    ) -> Result<Declared, ManifestReadingError> {
        read(manifest, candidates)
    }
}
