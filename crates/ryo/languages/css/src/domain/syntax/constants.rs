//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str = "bytes that are not UTF-8, which the parser cannot read: `syntax_error` reads what the language said of them";

/// The delim-token that opens a declaration's importance, `!` of `!important`.
pub(crate) const BANG: &str = "!";

/// The ident-token of a declaration's importance, matched in any case.
pub(crate) const IMPORTANT: &str = "important";
