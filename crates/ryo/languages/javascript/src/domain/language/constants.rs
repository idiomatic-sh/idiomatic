//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of javascript files.
pub(crate) const EXTENSIONS: &[&str] = &[
    PLAIN_EXTENSION,
    JSX_EXTENSION,
    MODULE_EXTENSION,
    SCRIPT_EXTENSION,
];

/// The ending of a plain javascript file.
pub(crate) const PLAIN_EXTENSION: &str = ".js";

/// The ending of a javascript file with JSX.
pub(crate) const JSX_EXTENSION: &str = ".jsx";

/// The ending of an ECMAScript module, read as a module whatever its syntax.
pub(crate) const MODULE_EXTENSION: &str = ".mjs";

/// The ending of a `CommonJS` script, read as a script rather than a module.
pub(crate) const SCRIPT_EXTENSION: &str = ".cjs";

/// The directories the language never reads: Node resolves installed packages from `node_modules` folders (Node.js docs, "Loading from `node_modules` folders").
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] =
    &[ryo::languages::SkippedDirectory("node_modules")];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "javascript";

/// Names that require evaluation, or anonymous bindings.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[ryo::languages::UnknownName {
    reason: ryo::languages::UnknownReason(0),
    text: ryo::languages::ReasonName("dynamic-or-anonymous"),
}];
