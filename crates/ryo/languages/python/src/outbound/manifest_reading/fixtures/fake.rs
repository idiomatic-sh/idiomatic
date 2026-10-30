//! The fake of the `manifest_reading` port: what the language's tests stand in for it with.

use super::super::errors::ManifestReadingError;
use super::super::interface::ManifestReadingPort;
use super::super::types::Claim;
use crate::domain::packages::types::{Declared, Naming, PackageName};
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// What the language's tests stand in for `project` with: it claims `pyproject.toml`, a manifest declares a package named by its text,
/// and text that opens a table it does not close, an unreadable manifest, is refused.
pub(crate) struct ManifestReadingFake;

impl ManifestReadingPort for ManifestReadingFake {
    fn manifests(&self) -> &'static [ManifestName] {
        &[ManifestName("pyproject.toml")]
    }

    fn claims(&self, path: &FilePath) -> Claim {
        if path.0.rsplit('/').next() == Some("pyproject.toml") {
            Claim::Reads
        } else {
            Claim::Skips
        }
    }

    fn declared(&self, manifest: &ManifestFile) -> Result<Declared, ManifestReadingError> {
        let text = &manifest.text;
        if text.starts_with("[project\n") {
            return Err(ManifestReadingError::Unreadable {
                reason: "the fake was told to fail".to_owned(),
            });
        }
        let name = text
            .strip_prefix("[project]\nname = \"")
            .and_then(|rest| rest.split('"').next())
            .map_or(Naming::Unnamed, |name| {
                Naming::Named(PackageName(name.into()))
            });
        Ok(Declared {
            name,
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: Vec::new(),
            sources: Vec::new(),
            overrides: Vec::new(),
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
