//! Package manifests as npm and Yarn write them: name, dependencies and workspaces.

use super::schemas::read;
use crate::domain::packages::constants::PACKAGE;
use crate::domain::packages::types::{Declared, ManifestPath};
use crate::outbound::manifest_reading::{
    errors::ManifestReadingError, interface::ManifestReadingPort,
};
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// The manifest files npm and Yarn write.
const CLAIMS: &[ManifestName] = &[ManifestName(PACKAGE)];

/// Reads a package manifest as npm and Yarn write it.
#[derive(Debug)]
pub(crate) struct ManifestReadingNpmAdapter {
    /// The manifests of the run, which member globs select from.
    candidates: Vec<FilePath>,
}

impl ManifestReadingNpmAdapter {
    /// The adapter over the manifests of the run.
    #[must_use]
    pub(crate) fn new(candidates: Vec<FilePath>) -> Self {
        Self { candidates }
    }
}

impl ManifestReadingPort for ManifestReadingNpmAdapter {
    fn claims(&self) -> &'static [ManifestName] {
        CLAIMS
    }

    fn about(&self, manifest: &ManifestFile) -> ManifestPath {
        ManifestPath(manifest.path.0.clone())
    }

    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
        read(manifest, &self.candidates)
    }
}
