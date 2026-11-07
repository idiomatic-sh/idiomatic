//! The pinned standard.
/// The standard the token vocabulary is taken from, and the parser that pinned its tokens.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str =
    "ECMAScript lexical token classes; Acorn 8.15.0 tokens and comments";
