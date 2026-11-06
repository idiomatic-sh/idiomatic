//! The names of this concept.
/// The dialect of every percent-format row.
pub(crate) const PERCENT_STRING: &str = "percent-string";
/// The representation of every percent-format row: the decoded string.
pub(crate) const DECODED_PERCENT_STRING: &str = "decoded-string";
/// The outcome of a string that parsed.
pub(crate) const PERCENT_PARSED: &str = "parsed";
/// The outcome of a string that is malformed.
pub(crate) const PERCENT_MALFORMED: &str = "malformed";
/// The outcome of a string built from dynamic input.
pub(crate) const PERCENT_UNKNOWN_INPUT: &str = "unknown-input";
/// The outcome of a string no grammar read.
pub(crate) const PERCENT_UNAVAILABLE: &str = "unavailable";
/// The error category of a string that is not malformed.
pub(crate) const NO_ERROR_CATEGORY: &str = "none";
/// The error category of an unmatched key parenthesis.
pub(crate) const UNMATCHED_KEY_PARENTHESES: &str = "unmatched-key-parentheses";
/// The error category of an incomplete format.
pub(crate) const INCOMPLETE_FORMAT: &str = "incomplete-format";
/// The error category of an unsupported format character.
pub(crate) const UNSUPPORTED_FORMAT_CHARACTER: &str = "unsupported-format-character";
/// The error category of a width or precision that is too large.
pub(crate) const WIDTH_OR_PRECISION_TOO_LARGE: &str = "width-or-precision-too-large";
/// The argument kind of a positional conversion.
pub(crate) const POSITIONAL: &str = "positional";
/// The argument kind of a conversion that selects a mapping key.
pub(crate) const MAPPING: &str = "mapping";
/// The mapping key of a positional conversion.
pub(crate) const NO_KEY: &str = "";
/// The text of a character that is absent.
pub(crate) const NO_CHARACTER: &str = "";
/// The mode of a width or precision a conversion does not write.
pub(crate) const NO_QUANTITY: &str = "none";
/// The mode of a width or precision written as a number.
pub(crate) const FIXED_QUANTITY: &str = "fixed";
/// The mode of a width or precision taken from an argument.
pub(crate) const STAR_QUANTITY: &str = "star";
/// The mode of a precision written as a bare dot.
pub(crate) const DOT_QUANTITY: &str = "dot";
