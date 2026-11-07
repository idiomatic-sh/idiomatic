//! The catalogs Bun writes in the root package manifest.

use super::schemas::read;
use crate::domain::packages::constants::PACKAGE;
use crate::domain::packages::types::{Declared, ManifestPath};
use crate::outbound::manifest_reading::{
    errors::ManifestReadingError, interface::ManifestReadingPort,
};
use ryo::languages::{ManifestFile, ManifestName};

/// The manifest files Bun keeps its catalogs in.
const CLAIMS: &[ManifestName] = &[ManifestName(PACKAGE)];

/// Reads the catalogs a Bun workspace declares in its root package manifest.
#[derive(Debug)]
pub(crate) struct ManifestReadingBunAdapter;

impl ManifestReadingPort for ManifestReadingBunAdapter {
    fn claims(&self) -> &'static [ManifestName] {
        CLAIMS
    }

    fn about(&self, manifest: &ManifestFile) -> ManifestPath {
        ManifestPath(manifest.path.0.clone())
    }

    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
        read(manifest)
    }
}
