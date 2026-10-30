//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the parser could not read as any token: `syntax_error` reads what the library said of it";

/// White space between tokens.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space the scanner gives no token for: `syntax_tree` holds it as it stands";
