//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of the files the language reads: TypeScript's, and JavaScript's, which TypeScript reads as well.
pub(crate) const EXTENSIONS: &[&str] = &[
    PLAIN_EXTENSION,
    JSX_EXTENSION,
    MODULE_EXTENSION,
    COMMONJS_EXTENSION,
    SCRIPT_EXTENSION,
    SCRIPT_JSX_EXTENSION,
    SCRIPT_MODULE_EXTENSION,
    SCRIPT_COMMONJS_EXTENSION,
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
/// The ending of the paths of plain JavaScript files.
pub(crate) const SCRIPT_EXTENSION: &str = ".js";
/// The ending of the paths of JavaScript files with JSX.
pub(crate) const SCRIPT_JSX_EXTENSION: &str = ".jsx";
/// The ending of the paths of JavaScript files that are always modules.
pub(crate) const SCRIPT_MODULE_EXTENSION: &str = ".mjs";
/// The ending of the paths of JavaScript files that are always `CommonJS`.
pub(crate) const SCRIPT_COMMONJS_EXTENSION: &str = ".cjs";
/// What marks a file of declarations: `index.d.ts`, `index.d.mts`, or, with arbitrary extensions, `styles.d.css.ts` (TypeScript docs,
/// "allowArbitraryExtensions").
pub(crate) const DECLARATIONS_MARK: &str = ".d.";
/// How the name of a file of declarations ends before a `.mts` or `.cts` extension.
pub(crate) const DECLARATIONS_END: &str = ".d";

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "typescript";
/// The compiler feature a file has when it is read as TypeScript, and lacks when it is read as JavaScript.
pub(crate) const TYPESCRIPT_SYNTAX: &str = "typescript.syntax";
/// Names requiring evaluation or an anonymous binding have no static name.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[ryo::languages::UnknownName {
    reason: ryo::languages::UnknownReason(0),
    text: ryo::languages::ReasonName("dynamic-or-anonymous"),
}];
