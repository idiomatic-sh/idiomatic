//! The constants of the `filters` part: how the plan writes each function a condition applies, and the words a
//! condition reads as constants.

/// `glob_any(text, glob)`.
pub(crate) const GLOB_ANY: &str = "glob_any";
/// `float_text_gte(left, right)`.
pub(crate) const FLOAT_TEXT_GTE: &str = "float_text_gte";
/// `numeric_gte(left, right)`.
pub(crate) const NUMERIC_GTE: &str = "numeric_gte";
/// `integer_successor(left, right)`.
pub(crate) const INTEGER_SUCCESSOR: &str = "integer_successor";
/// `natural_lt(left, right)`.
pub(crate) const NATURAL_LT: &str = "natural_lt";
/// `starts_with(text, prefix)`.
pub(crate) const STARTS_WITH: &str = "starts_with";
/// `ends_with(text, suffix)`.
pub(crate) const ENDS_WITH: &str = "ends_with";
/// `matches(text, pattern)`.
pub(crate) const MATCHES: &str = "matches";
/// `concat(a, b, ..)`.
pub(crate) const CONCAT: &str = "concat";
/// `pascal(text)`.
pub(crate) const PASCAL: &str = "pascal";
/// `camel(text)`.
pub(crate) const CAMEL: &str = "camel";
/// `snake(text)`.
pub(crate) const SNAKE: &str = "snake";
/// `kebab(text)`.
pub(crate) const KEBAB: &str = "kebab";
/// `lower(text)`.
pub(crate) const LOWER: &str = "lower";
/// The words a condition writes bare that are constants, never columns: `true` and `false`.
pub(crate) const TRUTH_WORDS: [&str; 2] = ["true", "false"];
