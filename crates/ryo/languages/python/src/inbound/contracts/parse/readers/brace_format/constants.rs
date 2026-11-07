//! The names of this concept.
/// The representation of every row: the decoded string.
pub(crate) const DECODED_STRING: &str = "decoded-string";
/// The outcome of a string that parsed.
pub(crate) const BRACE_PARSED: &str = "parsed";
/// The outcome of a string that is malformed.
pub(crate) const BRACE_MALFORMED: &str = "malformed";
/// The outcome of a string that is built from dynamic input.
pub(crate) const UNKNOWN_INPUT: &str = "unknown-input";
/// The outcome of a string no grammar read.
pub(crate) const BRACE_UNAVAILABLE: &str = "unavailable";
/// The error category of a string that is not malformed.
pub(crate) const NO_ERROR: &str = "none";
/// The error category of an unmatched bracket.
pub(crate) const UNMATCHED_BRACKET: &str = "unmatched-bracket";
/// The error category of a missing start bracket.
pub(crate) const MISSING_START_BRACKET: &str = "missing-start-bracket";
/// The error category of placeholders nested too deep.
pub(crate) const PLACEHOLDER_RECURSION_EXCEEDED: &str = "placeholder-recursion-exceeded";
/// The error category of an unknown conversion.
pub(crate) const UNKNOWN_CONVERSION: &str = "unknown-conversion";
/// The error category of an empty attribute.
pub(crate) const EMPTY_ATTRIBUTE: &str = "empty-attribute";
/// The error category of a missing right bracket.
pub(crate) const MISSING_RIGHT_BRACKET: &str = "missing-right-bracket";
/// The error category of a character after the right bracket.
pub(crate) const INVALID_CHARACTER_AFTER_RIGHT_BRACKET: &str =
    "invalid-character-after-right-bracket";
/// The context of a field written in the literal.
pub(crate) const LITERAL: &str = "literal";
/// The context of a field nested in a format specification.
pub(crate) const FORMAT_SPEC: &str = "format-spec";
/// The selector kind of an automatically numbered field.
pub(crate) const AUTOMATIC: &str = "automatic";
/// The selector kind of an explicitly indexed field.
pub(crate) const INDEX: &str = "index";
/// The selector kind of a keyword field.
pub(crate) const KEYWORD: &str = "keyword";
/// The selector name of a field that has none.
pub(crate) const NO_NAME: &str = "";
/// The parent ordinal of a field that has no parent.
pub(crate) const NO_PARENT: i64 = -1;
