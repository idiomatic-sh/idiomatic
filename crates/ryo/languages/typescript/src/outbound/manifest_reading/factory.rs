//! Builds the manifest reading adapters, one function each: one for each package manager, and one for the project configuration. A factory
//! builds nothing else, chooses nothing, and is called only by an edge. The adapters whose member globs select among the repository's
//! manifests are built for each run, over that run's.

use super::adapters::bun::adapter::ManifestReadingBunAdapter;
use super::adapters::npm::adapter::ManifestReadingNpmAdapter;
use super::adapters::pnpm::adapter::ManifestReadingPnpmAdapter;
use super::adapters::tsconfig::adapter::ManifestReadingTsconfigAdapter;
use ryo::languages::FilePath;

/// The adapter for npm, and Yarn, which writes package manifests alike, over the manifests of the run: what its workspace globs select
/// from.
#[must_use]
pub(crate) fn npm(candidates: Vec<FilePath>) -> impl super::interface::ManifestReadingPort {
    ManifestReadingNpmAdapter::new(candidates)
}

/// The adapter for pnpm workspaces, over the manifests of the run: what its package globs select from.
#[must_use]
pub(crate) fn pnpm(candidates: Vec<FilePath>) -> impl super::interface::ManifestReadingPort {
    ManifestReadingPnpmAdapter::new(candidates)
}

/// The adapter for Bun catalogs.
#[must_use]
pub(crate) fn bun() -> impl super::interface::ManifestReadingPort {
    ManifestReadingBunAdapter
}

/// The adapter for TypeScript project configuration.
#[must_use]
pub(crate) fn tsconfig() -> impl super::interface::ManifestReadingPort {
    ManifestReadingTsconfigAdapter
}
