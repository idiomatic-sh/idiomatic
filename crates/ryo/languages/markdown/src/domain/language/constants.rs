//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The endings of the paths of Markdown files.
pub(crate) const EXTENSIONS: [&str; 2] = [".md", ".markdown"];

/// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
/// ecosystem. None: `CommonMark` defines documents and no package, environment or build directory.
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "markdown";
