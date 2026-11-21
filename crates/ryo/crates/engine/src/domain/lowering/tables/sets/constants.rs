//! The constants of the `sets` part: what makes a glob or a regular expression more than literals.

/// The bytes a glob's syntax is written in (`ryo_matching`'s `globbing`: a star, one character, a class, a group of
/// alternatives, an escape). A glob that holds none of them matches its own text alone.
pub(crate) const GLOB_SYNTAX: &[u8] = b"*?[]{}\\";
/// What opens a glob's group of alternatives.
pub(crate) const GROUP_OPEN: char = '{';
/// What closes it.
pub(crate) const GROUP_CLOSE: char = '}';
/// What stands between two alternatives of a glob's group.
pub(crate) const GLOB_OR: char = ',';
/// What holds a regular expression to the start of the text.
pub(crate) const TEXT_START: char = '^';
/// What holds it to the end.
pub(crate) const TEXT_END: char = '$';
/// What opens a regular expression's group that captures nothing.
pub(crate) const PLAIN_GROUP: &str = "(?:";
/// What opens a group that captures.
pub(crate) const CAPTURING_GROUP: char = '(';
/// What closes either.
pub(crate) const GROUP_END: char = ')';
/// What stands between two alternatives of a regular expression.
pub(crate) const REGEX_OR: char = '|';
/// The bytes, beside letters and digits, a literal of a regular expression is written in here: none of them is syntax
/// outside a class.
pub(crate) const WORD_MARKS: &[u8] = b"_-";
