//! The `poetry` adapter of the manifest reading port: the `[tool.poetry]` tables of a `pyproject.toml`
//! read into the port's own types. The library's names stop here.

use ryo::languages::{FilePath, ManifestFile, ManifestName};

use super::schemas::declared;
use crate::domain::manifest_validation::types::MetadataDemand;
use crate::domain::packages::constants::PYPROJECT;
use crate::domain::packages::types::ManifestPath;
use crate::domain::patterns::constants::SEPARATOR;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;
use crate::outbound::manifest_reading::types::ManifestRead;

/// The `poetry` manifest reading adapter.
#[derive(Debug)]
pub(crate) struct ManifestReadingPoetryAdapter;

impl ManifestReadingPort for ManifestReadingPoetryAdapter {
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

    fn read(&self, manifest: &ManifestFile, demand: MetadataDemand) -> ManifestRead {
        if demand == MetadataDemand::Diagnostics {
            return ManifestRead {
                declaration: None,
                validation: None,
            };
        }
        ManifestRead {
            declaration: Some(declared(
                &ManifestPath(manifest.path.0.clone()),
                &manifest.text,
            )),
            validation: None,
        }
    }
}
