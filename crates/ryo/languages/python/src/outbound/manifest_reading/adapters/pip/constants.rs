//! The names of the `pip` adapter: the files it reads.

use ryo::languages::ManifestName;

/// The names of the files the adapter asks the repository for.
pub(super) const NAMES: &[ManifestName] = &[
    ManifestName("requirements.txt"),
    ManifestName("requirements-dev.txt"),
    ManifestName("requirements-test.txt"),
    ManifestName("requirements-prod.txt"),
    ManifestName("requirements-docs.txt"),
];

/// What a requirements file's name begins with, when it is one of several in a directory.
pub(super) const PREFIX: &str = "requirements-";

/// The directory whose text files are all requirements files.
pub(super) const DIRECTORY: &str = "requirements";

/// What a requirements file's name ends with.
pub(super) const SUFFIX: &str = ".txt";
