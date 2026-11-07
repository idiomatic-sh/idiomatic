//! The names of the `tokens` concept: the standard its tokens are named by, and why the relations read none of most of them.

/// The standard the tokens are named by: its section and the version pinned in `tests/fixtures/standard/`.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "CSS Syntax Module Level 3, §4 Tokenization, W3C Candidate Recommendation Draft 1 October 2026 (https://www.w3.org/TR/2026/CRD-css-syntax-3-20261001/)";

/// A token of a value or a selector.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const COMPONENT: &str = "a token of a value or a selector: no relation of the catalog models it; `syntax_tree` holds it as it stands";

/// White space.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space between the things the relations read; `syntax_tree` holds it as it stands";

/// A piece of punctuation.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str = "a piece of punctuation: syntax between the things the relations read, which read the construct it belongs to";

/// A token the tokenizer reports as an error.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const BROKEN: &str =
    "a token the tokenizer could not read: `syntax_error` reads what the library said of it";

/// A token the library never makes.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NEVER_MADE: &str = "cssparser makes no unicode-range-token: the standard makes one only where a grammar (`@font-face`'s `unicode-range`) asks, and the library reads `U+0-7F` as the ident, number and dimension tokens it is written with";
