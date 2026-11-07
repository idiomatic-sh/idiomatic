//! Builds the manifest reading adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `pep621` manifest reading adapter: `[project]` and the dependency groups of `pyproject.toml`.
#[must_use]
pub(crate) fn pep621() -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::pep621::adapter::ManifestReadingPep621Adapter
}

/// The `uv` manifest reading adapter: the workspace and the sources of `pyproject.toml`.
#[must_use]
pub(crate) fn uv() -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::uv::adapter::ManifestReadingUvAdapter
}

/// The `poetry` manifest reading adapter: the tables of `pyproject.toml` Poetry reads.
#[must_use]
pub(crate) fn poetry() -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::poetry::adapter::ManifestReadingPoetryAdapter
}

/// The `pip` manifest reading adapter: the requirements files, over `available`, every manifest of the run's repository, which the files a
/// requirements file includes are read from.
#[must_use]
pub(crate) fn pip(
    available: super::types::AvailableManifests,
) -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::pip::adapter::ManifestReadingPipAdapter::new(available.0)
}

/// The `pipenv` manifest reading adapter: the `Pipfile`.
#[must_use]
pub(crate) fn pipenv() -> impl crate::outbound::manifest_reading::interface::ManifestReadingPort {
    super::adapters::pipenv::adapter::ManifestReadingPipenvAdapter
}
