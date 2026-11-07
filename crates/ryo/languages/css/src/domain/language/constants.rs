//! The names of the `language` concept: the file names the language reads, and the values it cannot resolve.

/// The ending of the paths of CSS files, compared without regard to case.
pub(crate) const EXTENSION: &str = ".css";

/// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
/// ecosystem. None: CSS Syntax Module Level 3 defines a stylesheet's syntax and no package, environment or build directory, so every directory may hold stylesheets.
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "css";

/// The reasons a name column holds an unknown value: an `@import` brings in every rule of the stylesheet it names, and binds no name
/// it could list. The language's numbers are 48 to 63.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[ryo::languages::UnknownName {
    reason: ryo::languages::UnknownReason(48),
    text: ryo::languages::ReasonName("imports-every-rule"),
}];
