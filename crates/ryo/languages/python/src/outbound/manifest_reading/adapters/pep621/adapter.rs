//! The `pep621` adapter of the manifest reading port: the `[project]`, `[dependency-groups]` and `[build-system]` tables of a `pyproject.toml`
//! read into the port's own types. The library's names stop here.

use ryo::languages::{FilePath, ManifestFile, ManifestName};

use super::schemas::declared;
use crate::domain::packages::constants::PYPROJECT;
use crate::domain::packages::types::Declared;
use crate::domain::patterns::constants::SEPARATOR;
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;

/// The `pep621` manifest reading adapter.
#[derive(Debug)]
pub(crate) struct ManifestReadingPep621Adapter;

impl ManifestReadingPort for ManifestReadingPep621Adapter {
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
        declared(&manifest.text)
    }
}
