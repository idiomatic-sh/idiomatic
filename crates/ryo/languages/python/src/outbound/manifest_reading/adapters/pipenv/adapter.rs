//! The `pipenv` adapter of the manifest reading port: the `[packages]` and `[dev-packages]` tables of a `Pipfile` read into the port's own
//! types. The library's names stop here.

use ryo::languages::{FilePath, ManifestFile, ManifestName};

use super::constants::PIPFILE;
use super::schemas::declared;
use crate::domain::packages::types::{Declared, ManifestPath};
use crate::domain::patterns::constants::SEPARATOR;
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;

/// The `pipenv` manifest reading adapter.
#[derive(Debug)]
pub(crate) struct ManifestReadingPipenvAdapter;

impl ManifestReadingPort for ManifestReadingPipenvAdapter {
    fn manifests(&self) -> &'static [ManifestName] {
        &[ManifestName(PIPFILE)]
    }

    fn claims(&self, path: &FilePath) -> Claim {
        if path.0.rsplit(SEPARATOR).next() == Some(PIPFILE) {
            Claim::Reads
        } else {
            Claim::Skips
        }
    }

    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
        declared(&ManifestPath(manifest.path.0.clone()), &manifest.text)
    }
}
