//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of TOML files.
pub(crate) const EXTENSION: &str = ".toml";

/// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
/// ecosystem. None: the TOML specification defines a file format and no package, environment or build directory.
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "toml";

/// The names of the lock files written in TOML, which end in no `.toml`.
pub(crate) const LOCK_FILES: [&str; 3] = ["Cargo.lock", "uv.lock", "poetry.lock"];
