//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are Python.

use super::constants::EXTENSIONS;
use ryo::languages;

/// Whether `path` names a Python file: a module, a stub or a module run without a console.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if EXTENSIONS
        .iter()
        .any(|extension| path.0.ends_with(extension))
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// Declared Python project inputs belong to the project reader even under explicit source selection.
#[must_use]
pub(crate) fn primary_role(
    path: languages::SourcePath<'_>,
    manifests: &[languages::ManifestName],
) -> languages::PrimaryFileRole {
    languages::primary_role_from_manifests(path, manifests.iter().copied())
}
