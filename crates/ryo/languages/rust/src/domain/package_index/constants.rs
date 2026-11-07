//! The constants of the `package_index` concept: the text its service looks for.

use ryo::languages::ManifestName;

/// The file name of a package's manifest.
pub(crate) const MANIFEST: &str = crate::domain::paths::constants::MANIFEST;

/// The manifests this language reads.
pub(crate) const MANIFESTS: &[ManifestName] = &[ManifestName(MANIFEST)];
