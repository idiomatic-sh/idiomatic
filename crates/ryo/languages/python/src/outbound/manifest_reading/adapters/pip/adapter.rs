//! The `pip` adapter of the manifest reading port: a requirements file read into the port's own types. The names of the files it asks the
//! repository for are the ones every project shares; the shapes `requirements-*.txt` and `requirements/*.txt` it claims when the
//! repository hands them over.

use ryo::languages::{FilePath, ManifestFile, ManifestName};

use super::constants::{DIRECTORY, NAMES, PREFIX, SUFFIX};
use super::schemas::declared;
use crate::domain::manifest_validation::types::MetadataDemand;
use crate::domain::packages::types::ManifestPath;
use crate::domain::patterns::constants::SEPARATOR;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;
use crate::outbound::manifest_reading::types::ManifestRead;

/// The `pip` manifest reading adapter, over every manifest of the run's repository: the files a requirements file includes are read from them.
#[derive(Debug)]
pub(crate) struct ManifestReadingPipAdapter {
    /// Every manifest of the repository.
    available: Vec<ManifestFile>,
}

impl ManifestReadingPipAdapter {
    /// The adapter over every manifest of the run's repository.
    #[must_use]
    pub(crate) fn new(available: Vec<ManifestFile>) -> Self {
        Self { available }
    }
}

impl ManifestReadingPort for ManifestReadingPipAdapter {
    fn manifests(&self) -> &'static [ManifestName] {
        NAMES
    }

    fn claims(&self, path: &FilePath) -> Claim {
        let mut parts = path.0.rsplit(SEPARATOR);
        let name = parts.next().unwrap_or_default();
        let in_directory = parts.next() == Some(DIRECTORY);
        let by_name = NAMES.iter().any(|each| each.0 == name)
            || (name.starts_with(PREFIX) && name.ends_with(SUFFIX));
        if by_name || (in_directory && name.ends_with(SUFFIX)) {
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
                &self.available,
            )),
            validation: None,
        }
    }
}
