//! The `uv` adapter of the manifest reading port: the `[tool.uv.workspace]` and `[tool.uv.sources]` tables of a `pyproject.toml`
//! read into the port's own types. The library's names stop here.

use ryo::languages::{FilePath, ManifestFile, ManifestName};

use super::schemas::declared;
use crate::domain::packages::constants::PYPROJECT;
use crate::domain::packages::types::{Declared, ManifestPath};
use crate::domain::patterns::constants::SEPARATOR;
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;

/// The `uv` manifest reading adapter.
#[derive(Debug)]
pub(crate) struct ManifestReadingUvAdapter;

impl ManifestReadingPort for ManifestReadingUvAdapter {
    fn manifests(&self) -> &'static [ManifestName] {
        &[ManifestName(PYPROJECT)]
    }

    fn claims(&self, path: &FilePath) -> Claim {
        if path.0.rsplit(SEPARATOR).next() == Some(PYPROJECT) {
            Claim::Reads
        } else {
            Claim::Skips
        }
    }

    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
        declared(&ManifestPath(manifest.path.0.clone()), &manifest.text)
    }
}
