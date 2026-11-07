//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of go files.
pub(crate) const EXTENSION: &str = ".go";

/// The directories the language never reads: the go command does not match the packages of a `vendor` directory with `./...` (`go help packages`).
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] =
    &[ryo::languages::SkippedDirectory("vendor")];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "go";

/// The reasons the language declares for a name it cannot resolve statically, each named once, in the order of `Unresolved`: what a
/// row's unknown value says.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(32),
        text: ryo::languages::ReasonName("dynamic-callee"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(33),
        text: ryo::languages::ReasonName("declared-by-package"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(34),
        text: ryo::languages::ReasonName("dot-import"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(35),
        text: ryo::languages::ReasonName("not-text"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(36),
        text: ryo::languages::ReasonName("unnamed"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(37),
        text: ryo::languages::ReasonName("ambiguous-key"),
    },
];
