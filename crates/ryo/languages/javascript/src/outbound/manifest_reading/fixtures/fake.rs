//! A manifest reading port fake with the empty and refused contract cases.

use super::super::{errors::ManifestReadingError, interface::ManifestReadingPort};
use crate::domain::packages::types::{Declared, ManifestPath};
use ryo::languages::{FilePath, ManifestFile, ManifestName};

/// A manifest reading adapter that declares nothing for `{}` and refuses any other text.
pub(crate) struct ManifestReadingFake;
impl ManifestReadingPort for ManifestReadingFake {
    fn claims(&self) -> &'static [ManifestName] {
        &[ManifestName("package.json")]
    }

    fn about(&self, manifest: &ManifestFile) -> ManifestPath {
        ManifestPath(manifest.path.0.clone())
    }

    fn declared(
        &self,
        manifest: &ManifestFile,
        _candidates: &[FilePath],
    ) -> Result<Declared, ManifestReadingError> {
        if &*manifest.text.0 == "{}" {
            Ok(Declared::default())
        } else {
            Err(ManifestReadingError::Unreadable("fixture refusal".into()))
        }
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
