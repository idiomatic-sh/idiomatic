//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// A construct the relations read through its parts.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const THROUGH_PARTS: &str = "a construct the relations read through its parts: `entry` reads each part of a key, and the value beside it";

/// White space.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space between the things the relations read; `syntax_tree` holds it as it stands";

/// A piece of punctuation.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str = "a piece of punctuation: syntax between the things the relations read, which read the construct it belongs to";

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the parser could not read as any token: `syntax_error` reads what the library said of it";

/// The phase every `syntax_error` row names: the parser found the error.
pub(crate) const PARSE_PHASE: &str = "parse";

/// The impact every `syntax_error` row names: the recovered tree is incomplete.
pub(crate) const INCOMPLETE_IMPACT: &str = "incomplete";

/// The code every `syntax_error` row names: the parser gives no code.
pub(crate) const UNSPECIFIED_CODE: &str = "unspecified";

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";
