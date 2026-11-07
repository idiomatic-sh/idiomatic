//! Where this language-owned linter profile lives and where its runs make scratch directories.
use super::constants;
use ryo_benchmark::outbound::suite::{
    interface::{ScopedDirectory, SuitePort},
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::{Path, PathBuf};

/// Locate this language-owned linter profile from the Cargo manifest.
pub(super) fn directory() -> PathBuf {
    Path::new(constants::MANIFEST_DIRECTORY).join(constants::RUFF_DIRECTORY)
}

/// Make a scratch directory under the suite boundary's default location, removed when the value drops.
pub(super) fn default_scratch(
    suite: &dyn SuitePort,
    prefix: ArtifactText<'_>,
) -> Result<Box<dyn ScopedDirectory>, Error> {
    suite.scoped_directory(
        ArtifactPath(Path::new(constants::DEFAULT_SCRATCH_ROOT)),
        prefix,
    )
}
