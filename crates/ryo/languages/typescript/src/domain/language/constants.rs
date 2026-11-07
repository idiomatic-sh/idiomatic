//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of typescript files.
pub(crate) const EXTENSIONS: &[&str] = &[
    PLAIN_EXTENSION,
    JSX_EXTENSION,
    MODULE_EXTENSION,
    COMMONJS_EXTENSION,
];

/// The directories the language never reads: Node resolves installed packages from `node_modules` folders (Node.js docs, "Loading from `node_modules` folders").
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] =
    &[ryo::languages::SkippedDirectory("node_modules")];

/// The ending of the paths of TypeScript files with JSX.
pub(crate) const JSX_EXTENSION: &str = ".tsx";
/// The ending of the paths of TypeScript files that are always modules.
pub(crate) const MODULE_EXTENSION: &str = ".mts";
/// The ending of the paths of TypeScript files that are always `CommonJS`.
pub(crate) const COMMONJS_EXTENSION: &str = ".cts";
/// The ending of the paths of plain TypeScript files.
pub(crate) const PLAIN_EXTENSION: &str = ".ts";
/// What marks a file of declarations: `index.d.ts`, `index.d.mts`, or, with arbitrary extensions, `styles.d.css.ts` (TypeScript docs,
/// "allowArbitraryExtensions").
pub(crate) const DECLARATIONS_MARK: &str = ".d.";
/// How the name of a file of declarations ends before a `.mts` or `.cts` extension.
pub(crate) const DECLARATIONS_END: &str = ".d";

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "typescript";
/// Names requiring evaluation or an anonymous binding have no static name.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[ryo::languages::UnknownName {
    reason: ryo::languages::UnknownReason(0),
    text: ryo::languages::ReasonName("dynamic-or-anonymous"),
}];
