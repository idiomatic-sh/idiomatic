//! Builds the manifest reading adapters, one function each: one for each package manager. A factory
//! builds nothing else, chooses nothing, and is called only by an edge.

use super::adapters::bun::adapter::ManifestReadingBunAdapter;
use super::adapters::npm::adapter::ManifestReadingNpmAdapter;
use super::adapters::pnpm::adapter::ManifestReadingPnpmAdapter;

/// The adapter for npm, and Yarn, which writes package manifests alike.
#[must_use]
pub(crate) fn npm() -> impl super::interface::ManifestReadingPort {
    ManifestReadingNpmAdapter
}

/// The adapter for pnpm workspaces.
#[must_use]
pub(crate) fn pnpm() -> impl super::interface::ManifestReadingPort {
    ManifestReadingPnpmAdapter
}

/// The adapter for Bun catalogs.
#[must_use]
pub(crate) fn bun() -> impl super::interface::ManifestReadingPort {
    ManifestReadingBunAdapter
}
