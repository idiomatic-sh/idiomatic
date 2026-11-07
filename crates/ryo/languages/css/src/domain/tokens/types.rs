//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A token of a CSS syntax tree: one of the tokens CSS Syntax Level 3's tokenizer outputs (§4, `STANDARD`), named as it names
/// them. One closed list, each name once, in the standard's order.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[repr(u16)]
pub(crate) enum Token {
    /// `<ident-token>`, a name: `color`, `div`.
    #[strum(serialize = "ident-token")]
    Ident,
    /// `<function-token>`, a function's name and its `(`: `rgb(`.
    #[strum(serialize = "function-token")]
    Function,
    /// `<at-keyword-token>`, `@media`.
    #[strum(serialize = "at-keyword-token")]
    AtKeyword,
    /// `<hash-token>`, `#main`, `#123`.
    #[strum(serialize = "hash-token")]
    Hash,
    /// `<string-token>`, a string between quotes.
    #[strum(serialize = "string-token")]
    String,
    /// `<bad-string-token>`, a string a line end breaks.
    #[strum(serialize = "bad-string-token")]
    BadString,
    /// `<url-token>`, a `url(..)` written without quotes.
    #[strum(serialize = "url-token")]
    Url,
    /// `<bad-url-token>`, a `url(..)` the tokenizer could not read.
    #[strum(serialize = "bad-url-token")]
    BadUrl,
    /// `<delim-token>`, one code point that is no other token: `>`, `.`, `!`, and each of `~=`.
    #[strum(serialize = "delim-token")]
    Delim,
    /// `<number-token>`.
    #[strum(serialize = "number-token")]
    Number,
    /// `<percentage-token>`, a number with `%`.
    #[strum(serialize = "percentage-token")]
    Percentage,
    /// `<dimension-token>`, a number with a unit, `10px`.
    #[strum(serialize = "dimension-token")]
    Dimension,
    /// `<unicode-range-token>`, `U+0-7F`, which the tokenizer makes only where a grammar asks for one.
    #[strum(serialize = "unicode-range-token")]
    UnicodeRange,
    /// `<whitespace-token>`.
    #[strum(serialize = "whitespace-token")]
    Whitespace,
    /// `<CDO-token>`, `<!--`.
    #[strum(serialize = "cdo-token")]
    Cdo,
    /// `<CDC-token>`, `-->`.
    #[strum(serialize = "cdc-token")]
    Cdc,
    /// `<colon-token>`.
    #[strum(serialize = "colon-token")]
    Colon,
    /// `<semicolon-token>`.
    #[strum(serialize = "semicolon-token")]
    Semicolon,
    /// `<comma-token>`.
    #[strum(serialize = "comma-token")]
    Comma,
    /// `<[-token>`.
    #[strum(serialize = "[-token")]
    LeftSquare,
    /// `<]-token>`.
    #[strum(serialize = "]-token")]
    RightSquare,
    /// `<(-token>`.
    #[strum(serialize = "(-token")]
    LeftParenthesis,
    /// `<)-token>`.
    #[strum(serialize = ")-token")]
    RightParenthesis,
    /// `<{-token>`.
    #[strum(serialize = "{-token")]
    LeftCurly,
    /// `<}-token>`.
    #[strum(serialize = "}-token")]
    RightCurly,
}
