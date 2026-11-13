//! The text the `oxc_parser` adapter reads with.

use ryo::languages;

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: the language reads a file as UTF-8 text, and the parser reads it no further";

/// The two words of `import.meta`: the meta and the property TypeScript-ESTree gives a `MetaProperty` as identifiers.
pub(super) const IMPORT_META: [languages::NameText<'static>; 2] =
    [languages::NameText("import"), languages::NameText("meta")];
/// The two words of `new.target`, likewise.
pub(super) const NEW_TARGET: [languages::NameText<'static>; 2] =
    [languages::NameText("new"), languages::NameText("target")];

/// TypeScript retains this import-options key as a keyword token.
pub(super) const IMPORT_OPTIONS_KEY: &str = "with";

/// The extension Oxc reads plain TypeScript by.
pub(super) const PLAIN_EXTENSION: &str = "ts";
/// The extension Oxc reads TypeScript with JSX by.
pub(super) const JSX_EXTENSION: &str = "tsx";
/// The extension Oxc reads TypeScript that is always a module by.
pub(super) const MODULE_EXTENSION: &str = "mts";
/// The extension Oxc reads TypeScript that is always `CommonJS` by.
pub(super) const COMMONJS_EXTENSION: &str = "cts";

/// The name of ECMAScript's global regular-expression constructor: a string its call takes first is a pattern.
pub(super) const PATTERN_CONSTRUCTOR: &str = "RegExp";
/// The flags of a pattern whose call passes them as an argument the source does not settle.
pub(super) const UNKNOWN_FLAGS: &str = "unknown";
/// The flags of a pattern whose call passes no second argument: it states none.
pub(super) const NO_FLAGS: &str = "";
