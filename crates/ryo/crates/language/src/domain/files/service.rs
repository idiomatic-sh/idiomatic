//! Pure interpretation of declared project names; no parser, IO or second path registry.
use super::types::{PrimaryFileRole, SourcePath};
/// Classify a path against the provider's existing declared names.
#[must_use]
pub fn primary_role_from_manifests<'a>(
    path: SourcePath<'_>,
    mut names: impl Iterator<Item = &'a str>,
) -> PrimaryFileRole {
    let name = path.0.rsplit('/').next().unwrap_or(path.0);
    if names.any(|manifest| manifest == name) {
        PrimaryFileRole::Configuration
    } else {
        PrimaryFileRole::Source
    }
}
