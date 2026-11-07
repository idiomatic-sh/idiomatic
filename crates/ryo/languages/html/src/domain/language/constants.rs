//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of html files.
pub(crate) const EXTENSION: &str = ".html";

/// The short ending of the paths of html files, from the systems that kept three letters.
pub(crate) const SHORT_EXTENSION: &str = ".htm";

/// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
/// ecosystem. None: the WHATWG HTML Standard defines documents and no package, environment or build directory.
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "html";
