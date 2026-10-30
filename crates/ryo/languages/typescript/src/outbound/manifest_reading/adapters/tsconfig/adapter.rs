//! The project references of TypeScript project configuration.

use super::constants::MANIFEST;
use super::schemas::read;
use crate::domain::packages::types::{Declared, ManifestPath};
use crate::outbound::manifest_reading::{
    errors::ManifestReadingError, interface::ManifestReadingPort,
};
use ryo::languages::{ManifestFile, ManifestName};

/// The manifest files TypeScript's project configuration is written in.
const CLAIMS: &[ManifestName] = &[ManifestName(MANIFEST)];

/// Reads a project configuration's references, extensions and import paths.
#[derive(Debug)]
pub(crate) struct ManifestReadingTsconfigAdapter;

impl ManifestReadingPort for ManifestReadingTsconfigAdapter {
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
