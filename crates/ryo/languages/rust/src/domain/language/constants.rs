//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of Rust source files.
pub(crate) const EXTENSION: &str = ".rs";

/// The directories the language never reads: Cargo writes a package's build output to `target` (the Cargo Book, "Build cache").
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] =
    &[ryo::languages::SkippedDirectory("target")];

/// What the language is called, in listings.
pub(crate) const NAME: &str = "rust";
